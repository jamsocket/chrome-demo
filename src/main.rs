use axum::extract::ws::Message;
use axum::routing::get;
use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade},
    response::IntoResponse,
    Router,
};
use browser::Browser;
use clap::Parser;
use logging::init_logging;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use tokio::select;
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};

use crate::types::Command;

mod browser;
mod logging;
mod types;

#[derive(Parser)]
struct Opts {
    #[clap(short, long, default_value = "8080")]
    port: u16,

    #[clap(short, long, default_value = "https://jamsocket.com")]
    initial_url: String,

    #[clap(long, default_value = "http://localhost:4444")]
    webdriver_url: String,
}

async fn handle_socket(mut socket: WebSocket, browser: Arc<Browser>) {
    let mut frame_receiver = browser.frame_receiver();

    loop {
        select! {
            frame = socket.recv() => {
                match frame {
                    Some(Ok(frame)) => {
                        if let Message::Text(text) = frame {
                            tracing::info!(?text, "Got text.");
                            let command: Command = match serde_json::from_str(&text) {
                                Ok(command) => command,
                                Err(err) => {
                                    tracing::error!(?err, "Error parsing command.");
                                    continue;
                                }
                            };

                            tracing::info!(?command, "Got command.");
                            browser.send_command(command);
                        }
                    }
                    Some(Err(err)) => {
                        tracing::error!(?err, "Error receiving frame.");
                        return;
                    }
                    None => {
                        tracing::info!("WebSocket connection closed.");
                        return;
                    }
                }
            }
            _ = frame_receiver.changed() => {
                let frame = frame_receiver.borrow().clone();
                if let Some(frame) = frame {
                    let message = Message::Binary(frame);
                    if let Err(err) = socket.send(message).await {
                        tracing::error!(?err, "Error sending frame.");
                        return;
                    }
                }
            }
        }
    }
}

async fn ws_handler(
    State(browser): State<Arc<Browser>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, browser))
}

#[tokio::main]
async fn main() {
    init_logging();

    let args = Opts::parse();

    tracing::info!(%args.initial_url, "Starting browser.");
    let browser = Arc::new(Browser::new(&args.initial_url, args.webdriver_url.clone()));
    tracing::info!("Listening.");

    let app = Router::new()
        .fallback_service(ServeDir::new("./static").append_index_html_on_directories(true))
        .route("/ws", get(ws_handler))
        .with_state(browser)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, args.port);
    tracing::info!(%addr, "Listening.");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    let handle = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    select! {
        _ = handle => (),
        _ = tokio::signal::ctrl_c() => (),
    }
}
