#[macro_use] extern crate rocket;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_migrations;
#[macro_use] extern crate juniper;

mod config;
mod db;
mod plugins;
mod graphql;

use std::fs;
use clap::{AppSettings, Clap};
use serde::{Deserialize};
use figment::{Figment, providers::{Serialized, Format, Toml}};
use fern::colors::{Color, ColoredLevelConfig};
use rocket::http::Status;
use rocket::{response::content, State};
use rocket::fs::{FileServer};
use rocket::figment::{value::{Map, Value}, util::map};
use rocket_sync_db_pools::{database, diesel as rocket_diesel};

#[database("meiti")]
pub struct MeitiDb(rocket_diesel::SqliteConnection);

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Clap, Deserialize, serde::Serialize)]
#[clap(name = "Meiti Media Server",version = VERSION)]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(short, long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<u16>
}

fn setup_logger(config: &config::Config) -> Result<(), fern::InitError> {
    let colors_level = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::Blue)
        .debug(Color::White)
        .trace(Color::Magenta);

    fern::Dispatch::new()
        .level(log::LevelFilter::Debug)
        .chain(
            fern::Dispatch::new()
                .format(move |out, message, record| {
                    out.finish(format_args!(
                        "[{}][{}] {}",
                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                        colors_level.color(record.level()),
                        message
                    ))
                })
                .chain(std::io::stdout())
            )
        .chain(
            fern::Dispatch::new()
                .format(move |out, message, record| {
                    out.finish(format_args!(
                        "[{}][{}] {}",
                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                        record.level(),
                        message
                    ))
                })
                .chain(fern::DateBased::new(config.log_file_path.join("meiti.server."), "%Y-%m-%d.log"))
            )
        .apply()?;
    Ok(())
}

fn ensure_config_dirs(config: &config::Config) -> std::io::Result<()> {
    fs::create_dir_all(&config.log_file_path)?;
    fs::create_dir_all(&config.plugins_file_path)?;

    return Ok(());
}

#[rocket::get("/graphiql")]
fn graphiql() -> content::Html<String> {
    juniper_rocket::graphiql_source("/graphql", None)
}


#[rocket::get("/graphql?<request>")]
fn get_graphql_handler(
    context: MeitiDb,
    request: juniper_rocket::GraphQLRequest,
    schema: &State<graphql::schema::Schema>,
) -> juniper_rocket::GraphQLResponse {
    request.execute_sync(&*schema, &context)
}

#[rocket::post("/graphql", data = "<request>")]
fn post_graphql_handler<'a>(
    context: MeitiDb,
    request: juniper_rocket::GraphQLRequest,
    schema: &State<graphql::schema::Schema>,
) -> juniper_rocket::GraphQLResponse {
    request.execute_sync(&*schema, &context)
}

#[get("/")]
fn root_redirect() -> rocket::response::Redirect {
    return rocket::response::Redirect::temporary("/web/");
}

#[get("/health")]
fn health() -> Status {
    Status::Accepted
}

async fn run_migrations(rocket: rocket::Rocket<rocket::Build>) -> rocket::Rocket<rocket::Build> {
    // This macro from `diesel_migrations` defines an `embedded_migrations`
    // module containing a function named `run`. This allows the example to be
    // run and tested without any outside setup of the database.
    embed_migrations!();

    let conn = MeitiDb::get_one(&rocket).await.expect("Failed to connect to the database for migrations");
    conn.run(|c| embedded_migrations::run(c)).await.expect("Failed to run migrations");

    rocket
}

#[rocket::main]
async fn main() {
    let proj_dirs: Option<directories::ProjectDirs> = directories::ProjectDirs::from("tv", "Meiti",  "Meiti Server");

    let opts: Opts = Opts::parse();

    let mut config_file_path: std::path::PathBuf = std::path::PathBuf::from(proj_dirs.unwrap().config_dir());
    config_file_path.push("meiti.toml");

    // If we don't have an existing config file, just write the defaults to it
    if !config_file_path.as_path().exists() {
        fs::create_dir_all(&config_file_path).expect("Unable to create configuration directory");

        let mut config_file_dir = config_file_path.clone();
        config_file_dir.pop();

        let serialized_defaults = toml::to_string(&config::Config::default()).expect("Unable to serialize default configuration");
        fs::write(&config_file_path, serialized_defaults).expect("Unable to write file");
        log::info!("Wrote default configuration to {:?}", &config_file_path)
    }

    let figment = Figment::from(Toml::file(config_file_path))
        .merge(Serialized::defaults(opts));

    let config: config::Config = figment.extract::<config::Config>().expect("The provided configuration is invalid");

    // Ensure the directories we need exist before going further.
    ensure_config_dirs(&config).expect("Unable to create required directories");

    setup_logger(&config).expect("failed to initialize logging");

    log::info!("Meiti Media Server v{}", VERSION);
    log::info!("Using port {:?}", config.port);
    log::info!("Using log path {:?}", config.log_file_path);
    log::info!("Using plugins path {:?}", config.plugins_file_path);

    // Only enable Sentry reporting if the user explicitely agreed to it
    if config.sentry {
        log::info!("Initializing Sentry for reporting crashes");

        let _guard = sentry::init(("https://a191a66759744bc39e79c122ed69da3b@o725130.ingest.sentry.io/5948021", sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        }));
    }

    let mut plugin_manager = plugins::PluginManager::new();

    unsafe {
        plugin_manager.load_all_plugins(&config).expect("Failed to load plugins");
    }

    let web_resources_dir = plugin_manager.get_web_client_resources();

    log::info!("Serving web client from {:?}", web_resources_dir.as_path());

    let db: Map<_, Value> = map! {
        "url" => config.database_file_path.to_str().unwrap().into(),
        "pool_size" => 10.into()
    };

    let rocket_figment = rocket::Config::figment()
        .merge(("databases", map!["meiti" => db]))
        .merge(("port", &config.port));

    let rocket = rocket::custom(rocket_figment)
        .attach(MeitiDb::fairing())
        .attach(rocket::fairing::AdHoc::on_ignite("Running database migrations", run_migrations))
        .manage(
            graphql::schema::Schema::new(
                graphql::schema::Query,
                juniper::EmptyMutation::<MeitiDb>::new(),
                juniper::EmptySubscription::<MeitiDb>::new()
        ))
        .mount("/web/", FileServer::from(web_resources_dir.as_path()).rank(1))
        .mount("/", routes![
            graphiql,
            get_graphql_handler,
            post_graphql_handler,
            root_redirect,
            health
        ]);

    rocket.launch().await.expect("Failed to launch the web server");

    log::info!("Shutting down the server");
}
