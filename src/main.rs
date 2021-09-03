#[macro_use] extern crate rocket;

mod config;
mod plugins;

use std::fs;
use clap::{AppSettings, Clap};
use serde::{Deserialize};
use figment::{Figment, providers::{Serialized}};
use fern::colors::{Color, ColoredLevelConfig};
use rocket::http::Status;

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

#[get("/health")]
fn health() -> Status {
    Status::Accepted
}

#[rocket::main]
async fn main() {
    let opts: Opts = Opts::parse();

    let figment = Figment::from(Serialized::defaults(config::Config::default()))
        .merge(Serialized::defaults(opts));

    let config: config::Config = figment.extract::<config::Config>().expect("The provided configuration is invalid");

    // Ensure the directories we need exist before going further.
    ensure_config_dirs(&config).expect("Unable to create required directories");

    setup_logger(&config).expect("failed to initialize logging");

    log::info!("Meiti Media Server v{}", VERSION);
    log::info!("Using port {:?}", config.port);
    log::info!("Using log path {:?}", config.log_file_path);
    log::info!("Using plugins path {:?}", config.plugins_file_path);

    let mut plugin_manager = plugins::PluginManager::new();

    for entry in fs::read_dir(config.plugins_file_path).unwrap() {
        if let Ok(path) = entry {
                unsafe {
                    plugin_manager.load_plugin(path.path()).expect("Failed to load plugin");
                }
            }
        }

    let routes = plugin_manager.register_routes();

    let rocket = rocket::build()
        .mount("/", routes![health])
        .mount("/", routes);

    rocket.launch().await.expect("Failed to launch the web server");

    log::info!("Shutting down the server");
}
