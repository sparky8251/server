
use fern::colors::{Color, ColoredLevelConfig};
use actix_web::{get, App, HttpServer, Responder, HttpResponse};

fn setup_logger() -> Result<(), fern::InitError> {
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
                        "[{}][{}][{}] {}",
                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                        record.target(),
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
                        "[{}][{}][{}] {}",
                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                        record.target(),
                        record.level(),
                        message
                    ))
                })
                .chain(fern::DateBased::new("meiti.server.", "%Y-%m-%d.log"))
            )
        .apply()?;
    Ok(())
}

#[get("/healh")]
async fn health() -> impl Responder {
    HttpResponse::Ok()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    setup_logger().expect("failed to initialize logging.");

    log::info!("Meiti Media Server v{}", VERSION);

    HttpServer::new(|| {
        App::new()
            .service(health)
    })
    .bind("127.0.0.1:23400")?
    .run()
    .await
}
