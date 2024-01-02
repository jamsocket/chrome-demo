use crate::browser_actor::{
    AddClientMessage, BrowserActor, GetConnectionInfo, RemoveClientMessage,
};
use actix::{Actor, Addr, AsyncContext, Handler, Message, StreamHandler};
use actix_web::web::Json;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize)]
pub struct ConnectionInfo {
    pub active_connections: u32,
    pub seconds_inactive: u32,
    pub listening: bool,
}
pub struct WebsocketConnection {
    browser: Addr<BrowserActor>,
}

impl Actor for WebsocketConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.browser.do_send(AddClientMessage(ctx.address()));
    }
}

#[derive(Deserialize, Debug, Message)]
#[rtype(result = "()")]
#[serde(tag = "action")]
pub enum BrowserAction {
    #[serde(rename = "key")]
    KeyPress { key: String },
    #[serde(rename = "click")]
    Click { x: f64, y: f64 },
    #[serde(rename = "navigate")]
    Navigate { url: String },
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SendScreenshot(pub Vec<u8>);

#[derive(Message)]
#[rtype(result = "()")]
pub struct SendUrl(pub String);

impl Handler<SendUrl> for WebsocketConnection {
    type Result = ();

    fn handle(&mut self, msg: SendUrl, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(msg.0);
    }
}

impl Handler<SendScreenshot> for WebsocketConnection {
    type Result = ();

    fn handle(&mut self, msg: SendScreenshot, ctx: &mut Self::Context) -> Self::Result {
        ctx.binary(msg.0);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebsocketConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => {
                let value: BrowserAction = serde_json::from_str(&text).unwrap();

                tracing::info!(?value, "Got event from browser.");

                self.browser.do_send(value);
            }
            Ok(ws::Message::Close(_)) => {
                self.browser.do_send(RemoveClientMessage(ctx.address()));
            }
            value => {
                tracing::warn!(?value, "Unexpected websocket message.");
            }
        }
    }
}

pub async fn index(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let browser: &Addr<BrowserActor> = req.app_data().unwrap();

    let resp = ws::start(
        WebsocketConnection {
            browser: browser.clone(),
        },
        &req,
        stream,
    );
    resp
}

pub async fn status(req: HttpRequest) -> Result<Json<ConnectionInfo>, Error> {
    let browser: &Addr<BrowserActor> = req.app_data().unwrap();

    let connection_info = browser.send(GetConnectionInfo).await.unwrap();

    Ok(Json(connection_info))
}
