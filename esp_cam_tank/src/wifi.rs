use anyhow::{Context, Result};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi};

pub type Link = BlockingWifi<EspWifi<'static>>;

pub fn connect(modem: Modem<'static>, ssid: &str, password: &str) -> Result<Link> {
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let mut wifi = BlockingWifi::wrap(EspWifi::new(modem, sys_loop.clone(), Some(nvs))?, sys_loop)?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid.try_into().ok().context("ssid longer than 31 bytes")?,
        password: password
            .try_into()
            .ok()
            .context("password longer than 63 bytes")?,
        auth_method: AuthMethod::WPAWPA2Personal,
        ..ClientConfiguration::default()
    }))?;

    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;

    let ip = wifi.wifi().sta_netif().get_ip_info()?.ip;
    log::info!("wifi up, ip: {ip}");
    Ok(wifi)
}
