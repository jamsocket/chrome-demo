use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, SystemTime},
};

use crate::websocket::{
    BrowserAction, ConnectionInfo, SendScreenshot, SendUrl, WebsocketConnection,
};
use actix::{Actor, Addr, AsyncContext, Context, Handler, Message, MessageResult};
use headless_chrome::protocol::browser::Bounds;
use headless_chrome::{
    browser::{
        default_executable,
        tab::{get_key_definition, point::Point},
    },
    protocol::page::ScreenshotFormat,
    Browser, LaunchOptions, Tab,
};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

const TICK_MILLIS: u64 = 100;

#[derive(Message)]
#[rtype("()")]
struct Tick;

pub struct BrowserActor {
    listeners: HashSet<Addr<WebsocketConnection>>,
    idle_since: Option<SystemTime>,
    _browser: Browser,
    tab: Arc<Tab>,
    screen_data: Vec<u8>,
    url: String,
}

impl BrowserActor {
    pub fn new(initial_url: &str) -> Self {
        let exec = default_executable().unwrap();

        tracing::info!(?exec, "Creating browser.");

        let browser = Browser::new(LaunchOptions {
            idle_browser_timeout: Duration::from_secs(3600),
            path: Some(exec),
            sandbox: false,
            ..Default::default()
        })
        .expect("Couldn't create browser.");

        tracing::info!("Waiting on tab.");
        let tab = browser.wait_for_initial_tab().unwrap();
        tab.set_bounds(Bounds::Normal {
            left: Some(0),
            top: Some(0),
            width: Some(WIDTH),
            height: Some(HEIGHT),
        })
        .unwrap();
        tracing::info!("Got tab.");

        tab.navigate_to(initial_url).unwrap();

        let screen_data = tab
            .capture_screenshot(ScreenshotFormat::PNG, None, true)
            .unwrap();
        tracing::info!("Got screenshot. {}", screen_data.len());

        BrowserActor {
            listeners: HashSet::default(),
            _browser: browser,
            screen_data,
            tab,
            url: initial_url.to_string(),
            idle_since: Some(SystemTime::now()),
        }
    }

    pub fn update(&mut self) {
        let screen_data = self
            .tab
            .capture_screenshot(ScreenshotFormat::PNG, None, true)
            .unwrap();

        let url = self.tab.get_url();

        if url != self.url {
            self.url = url;
            for listener in &self.listeners {
                listener.do_send(SendUrl(self.url.clone()))
            }
            tracing::info!("Updated URL");
        }

        if screen_data != self.screen_data {
            self.screen_data = screen_data;
            for listener in &self.listeners {
                listener.do_send(SendScreenshot(self.screen_data.clone()))
            }
            tracing::info!("Updated Screenshot");
        }
    }
}

impl Actor for BrowserActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.notify_later(Tick, Duration::from_millis(TICK_MILLIS));
    }
}

#[derive(Message)]
#[rtype("()")]
pub struct AddClientMessage(pub Addr<WebsocketConnection>);

#[derive(Message)]
#[rtype("()")]
pub struct RemoveClientMessage(pub Addr<WebsocketConnection>);

#[derive(Message)]
#[rtype("ConnectionInfo")]
pub struct GetConnectionInfo;

impl Handler<AddClientMessage> for BrowserActor {
    type Result = ();

    fn handle(&mut self, AddClientMessage(listener): AddClientMessage, _ctx: &mut Self::Context) {
        listener.do_send(SendScreenshot(self.screen_data.clone()));
        listener.do_send(SendUrl(self.url.clone()));

        self.listeners.insert(listener);
        self.idle_since = None;
        tracing::info!(num_clients=%self.listeners.len(), "Added client.");
    }
}

impl Handler<RemoveClientMessage> for BrowserActor {
    type Result = ();

    fn handle(
        &mut self,
        RemoveClientMessage(listener): RemoveClientMessage,
        _ctx: &mut Self::Context,
    ) {
        self.listeners.remove(&listener);

        if self.listeners.is_empty() {
            self.idle_since = Some(SystemTime::now());
            tracing::info!("No listeners.");
        }

        tracing::info!(num_clients=%self.listeners.len(), "Removed client.");
    }
}

impl Handler<GetConnectionInfo> for BrowserActor {
    type Result = MessageResult<GetConnectionInfo>;

    fn handle(&mut self, _: GetConnectionInfo, _ctx: &mut Self::Context) -> Self::Result {
        let active_connections = self.listeners.len() as _;
        let seconds_inactive = self
            .idle_since
            .map(|d| SystemTime::now().duration_since(d).unwrap().as_secs() as _)
            .unwrap_or_default();

        MessageResult(ConnectionInfo {
            active_connections,
            seconds_inactive,
            listening: true,
        })
    }
}

impl Handler<Tick> for BrowserActor {
    type Result = ();

    fn handle(&mut self, _: Tick, ctx: &mut Self::Context) -> Self::Result {
        tracing::info!("Tick");
        self.update();

        ctx.notify_later(Tick, Duration::from_millis(TICK_MILLIS));
    }
}

impl Handler<BrowserAction> for BrowserActor {
    type Result = ();

    fn handle(&mut self, msg: BrowserAction, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            BrowserAction::KeyPress { key } => {
                if let Ok(key) = get_key_definition(&key) {
                    self.tab.press_key(&key).unwrap();
                }
            }
            BrowserAction::Click { x, y } => {
                self.tab.click_point(Point { x, y }).unwrap();
            }
            BrowserAction::Navigate { url } => {
                self.tab.navigate_to(&url).unwrap();
            }
        }

        self.update();
    }
}
