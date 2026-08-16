use std::sync::mpsc::{Receiver, SyncSender};
use std::thread::Builder;
use std::time::Duration;

use anyhow::{Context, Result};
use esp_idf_svc::hal::cpu::Core;
use esp_idf_svc::io::EspIOError;
use esp_idf_svc::ws::client::{
    EspWebSocketClient, EspWebSocketClientConfig, WebSocketEvent, WebSocketEventType,
};
use esp_idf_svc::ws::FrameType;

use crate::proto::{self, Direction};
use crate::tasks::{self, Priority};

const TX_STACK_SIZE: usize = 6144;
const CLIENT_TASK_STACK: usize = 6144;
const CLIENT_TASK_PRIO: u8 = Priority::WsClient as u8;
const CLIENT_BUFFER_SIZE: usize = 1024;
const SEND_TIMEOUT: Duration = Duration::from_secs(5);

pub fn spawn(url: &str, cmds: SyncSender<Direction>, frames: Receiver<Vec<u8>>) -> Result<()> {
    let config = EspWebSocketClientConfig {
        task_prio: CLIENT_TASK_PRIO,
        task_stack: CLIENT_TASK_STACK,
        buffer_size: CLIENT_BUFFER_SIZE,
        ..Default::default()
    };
    let client = EspWebSocketClient::new(url, &config, SEND_TIMEOUT, move |event| {
        handle_event(event, &cmds)
    })
    .context("ws client init")?;

    tasks::configure(c"ws_tx", Priority::WsTx, Core::Core0)?;
    Builder::new()
        .stack_size(TX_STACK_SIZE)
        .spawn(move || {
            let mut client = client;
            while let Ok(frame) = frames.recv() {
                if !client.is_connected() {
                    continue;
                }
                if let Err(e) = client.send(FrameType::Text(false), &frame) {
                    log::warn!("ws send: {e}");
                }
            }
            log::warn!("ws tx loop ended");
        })
        .context("spawn ws tx thread")?;
    Ok(())
}

fn handle_event(
    event: &std::result::Result<WebSocketEvent<'_>, EspIOError>,
    cmds: &SyncSender<Direction>,
) {
    match event {
        Ok(WebSocketEvent {
            event_type: WebSocketEventType::Text(text),
            ..
        }) => match proto::Direction::parse(text.as_bytes()) {
            Some(dir) => {
                let _ = cmds.send(dir);
            }
            None => log::warn!("ws unparsed frame: {text}"),
        },
        Ok(WebSocketEvent {
            event_type: WebSocketEventType::Connected,
            ..
        }) => log::info!("ws connected"),
        Ok(WebSocketEvent {
            event_type: WebSocketEventType::Disconnected | WebSocketEventType::Closed,
            ..
        }) => {
            log::warn!("ws disconnected");
            let _ = cmds.send(Direction::Stop);
        }
        Err(e) => log::warn!("ws error: {e}"),
        _ => {}
    }
}
