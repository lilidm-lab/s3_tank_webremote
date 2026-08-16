use core::ffi::CStr;

use anyhow::Result;
use esp_idf_svc::hal::cpu::Core;
use esp_idf_svc::hal::task::thread::ThreadSpawnConfiguration;

pub enum Priority {
    WsClient = 14,
    Motor = 12,
    WsTx = 8,
    Camera = 6,
}

// esp_pthread config is consumed by the next pthread_create on this thread,
// so configure() must run immediately before Builder::spawn (sequential spawns only).
pub fn configure(name: &'static CStr, priority: Priority, core: Core) -> Result<()> {
    let cfg = ThreadSpawnConfiguration {
        name: Some(name),
        priority: priority as u8,
        pin_to_core: Some(core),
        ..Default::default()
    };
    Ok(cfg.set()?)
}
