use crate::types::Command;
use headless_chrome::{
    browser::tab::point::Point, protocol::cdp::Page::CaptureScreenshotFormatOption, Browser,
};
use std::{sync::mpsc, time::Duration};
use tokio::sync::watch;

const TICK_MILLIS: u64 = 100;

pub struct HeadlessBrowser {
    frame_receiver: watch::Receiver<Option<Vec<u8>>>,
    _handle: std::thread::JoinHandle<()>,
    command_sender: mpsc::Sender<Command>,
}

fn browser_loop(sender: watch::Sender<Option<Vec<u8>>>, url_receiver: mpsc::Receiver<Command>) {
    let browser = Browser::default().unwrap();

    let tab = browser.new_tab().unwrap();

    tab.navigate_to("about:blank").unwrap();

    let mut last_screen_data: Option<Vec<u8>> = None;

    loop {
        let screen_data = tab
            .capture_screenshot(CaptureScreenshotFormatOption::Png, None, None, true)
            .unwrap();

        match url_receiver.try_recv() {
            Ok(Command::Navigate { url: new_url }) => {
                tracing::info!(%new_url, "Navigating to new URL.");
                tab.navigate_to(&new_url).unwrap();
            }

            Ok(Command::Click { x, y }) => {
                tracing::info!(%x, %y, "Clicking.");
                tab.click_point(Point { x, y }).unwrap();
                tracing::info!("Clicked.");
            }

            Ok(Command::Key { key }) => {
                tracing::info!(%key, "Typing.");
                tab.type_str(&key).unwrap();
            }

            Err(_) => (),
        }

        if Some(&screen_data) != last_screen_data.as_ref() {
            last_screen_data = Some(screen_data.clone());

            // A broadcast sender will error if there is nothing to receive it,
            // but we want to ignore that error.
            let _ = sender.send(Some(screen_data));
        }

        std::thread::sleep(Duration::from_millis(TICK_MILLIS));
    }
}

impl HeadlessBrowser {
    pub fn new(initial_url: &str) -> Self {
        let (frame_sender, frame_receiver) = watch::channel(None);
        let (command_sender, command_receiver) = mpsc::channel();
        let handle = std::thread::spawn(move || browser_loop(frame_sender, command_receiver));

        command_sender
            .send(Command::Navigate {
                url: initial_url.to_string(),
            })
            .unwrap();

        HeadlessBrowser {
            _handle: handle,
            frame_receiver,
            command_sender,
        }
    }

    pub fn frame_receiver(&self) -> watch::Receiver<Option<Vec<u8>>> {
        self.frame_receiver.clone()
    }

    pub fn send_command(&self, command: Command) {
        self.command_sender.send(command).unwrap();
    }
}
