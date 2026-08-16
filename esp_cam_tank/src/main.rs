use std::sync::mpsc::sync_channel;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use esp_idf_svc::hal::peripherals::Peripherals;

mod camera;
mod motor;
mod proto;
mod tasks;
mod wifi;
mod ws;

const CMD_CHANNEL_CAP: usize = 8;
const FRAME_CHANNEL_CAP: usize = 2;
const MAIN_POLL: Duration = Duration::from_secs(3600);

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let ssid = env_or("WIFI_SSID", option_env!("WIFI_SSID"))?;
    let password = env_or("WIFI_PASS", option_env!("WIFI_PASS"))?;
    let url = env_or("WS_URL", option_env!("WS_URL"))?;

    let (cmd_tx, cmd_rx) = sync_channel(CMD_CHANNEL_CAP);
    let (frame_tx, frame_rx) = sync_channel(FRAME_CHANNEL_CAP);

    camera::spawn(frame_tx)?;

    let Peripherals {
        modem, pins, ledc, ..
    } = Peripherals::take()?;
    // Motor pins must reach a defined low/stop state before the Wi-Fi blocking
    // connect (or its timeout): until then GPIO1/2 float and GPIO39-42 sit in
    // JTAG pull-up state, which can drive a motor by itself.
    let tank = motor::Tank::new(pins, ledc)?;
    motor::spawn(tank, cmd_rx)?;

    let _link = wifi::connect(modem, ssid, password)?;

    ws::spawn(url, cmd_tx, frame_rx)?;

    log::info!("tank online");
    loop {
        thread::sleep(MAIN_POLL);
    }
}

fn env_or(name: &str, value: Option<&'static str>) -> Result<&'static str> {
    value.with_context(|| format!("{name} not set at build time"))
}
