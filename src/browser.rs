use crate::types::Command;
use fantoccini::{
    actions::{InputSource, MouseActions, PointerAction, MOUSE_BUTTON_LEFT},
    ClientBuilder,
};
use std::time::Duration;
use tokio::sync::{mpsc, watch};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TICK_MILLIS: u64 = 100;

pub struct Browser {
    frame_receiver: watch::Receiver<Option<Vec<u8>>>,
    _handle: tokio::task::JoinHandle<()>,
    command_sender: mpsc::Sender<Command>,
}

async fn browser_loop(
    sender: watch::Sender<Option<Vec<u8>>>,
    mut url_receiver: mpsc::Receiver<Command>,
    webdriver_url: String,
) {
    let browser = ClientBuilder::native()
        .connect(&webdriver_url)
        .await
        .expect("failed to connect to WebDriver");

    browser.goto("about:blank").await.unwrap();

    browser.set_window_size(WIDTH, HEIGHT).await.unwrap();

    let mut last_screen_data: Option<Vec<u8>> = None;

    loop {
        let screen_data = browser.screenshot().await.unwrap();

        match url_receiver.try_recv() {
            Ok(Command::Navigate { url: new_url }) => {
                tracing::info!(%new_url, "Navigating to new URL.");
                browser.goto(&new_url).await.unwrap();
            }

            Ok(Command::Click { x, y }) => {
                let actions = MouseActions::new("mouse".to_string())
                    .then(PointerAction::MoveTo {
                        duration: None,
                        x,
                        y,
                    })
                    .then(PointerAction::Down {
                        button: MOUSE_BUTTON_LEFT,
                    })
                    .then(PointerAction::Up {
                        button: MOUSE_BUTTON_LEFT,
                    });

                tracing::info!(%x, %y, "Clicking.");
                browser.perform_actions(actions).await.unwrap();
                tracing::info!("Clicked.");
            }

            Ok(Command::Key { key }) => {
                tracing::info!(%key, "Typing.");
            }

            Err(_) => (),
        }

        if Some(&screen_data) != last_screen_data.as_ref() {
            last_screen_data = Some(screen_data.clone());

            // A broadcast sender will error if there is nothing to receive it,
            // but we want to ignore that error.
            let _ = sender.send(Some(screen_data));
        }

        tokio::time::sleep(Duration::from_millis(TICK_MILLIS)).await;
    }
}

impl Browser {
    pub fn new(initial_url: &str, webdriver_url: String) -> Self {
        let (frame_sender, frame_receiver) = watch::channel(None);
        let (command_sender, command_receiver) = mpsc::channel(30);
        let handle = tokio::spawn(browser_loop(frame_sender, command_receiver, webdriver_url));

        command_sender
            .try_send(Command::Navigate {
                url: initial_url.to_string(),
            })
            .unwrap();

        Browser {
            _handle: handle,
            frame_receiver,
            command_sender,
        }
    }

    pub fn frame_receiver(&self) -> watch::Receiver<Option<Vec<u8>>> {
        self.frame_receiver.clone()
    }

    pub fn send_command(&self, command: Command) {
        self.command_sender.try_send(command).unwrap();
    }
}
