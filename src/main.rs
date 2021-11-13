use actix::Actor;
use actix_files::Files;
use actix_web::{web, App, HttpServer};
use browser_actor::BrowserActor;
use logging::init_logging;
use websocket::{index, status};

mod browser_actor;
mod logging;
mod websocket;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_logging();

    let mut args = std::env::args();
    args.next();
    let initial_url = args.next().expect("Expected a single argument, the initial URL.");

    tracing::info!(%initial_url, "Starting browser.");
    let browser = BrowserActor::new(&initial_url).start();
    tracing::info!("Listening.");

    HttpServer::new(move || {
        App::new()
            .app_data(browser.clone())
            .route("/ws", web::get().to(index))
            .route("/status", web::get().to(status))
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
