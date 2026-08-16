use std::sync::mpsc::SyncSender;
use std::thread::Builder;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use esp_cam::espcam::Camera;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_svc::hal::cpu::Core;
use esp_idf_svc::sys;
use esp_idf_svc::sys::camera as cam;

use crate::proto;
use crate::tasks::{self, Priority};

const CAM_PIN_SIOD: u8 = 4;
const CAM_PIN_SIOC: u8 = 5;
const CAM_PIN_VSYNC: u8 = 6;
const CAM_PIN_HREF: u8 = 7;
const CAM_PIN_PCLK: u8 = 13;
const CAM_PIN_D0: u8 = 11;
const CAM_PIN_D1: u8 = 9;
const CAM_PIN_D2: u8 = 8;
const CAM_PIN_D3: u8 = 10;
const CAM_PIN_D4: u8 = 12;
const CAM_PIN_D5: u8 = 18;
const CAM_PIN_D6: u8 = 17;
const CAM_PIN_D7: u8 = 16;
const CAM_PIN_XCLK: u8 = 15;

const FRAME_INTERVAL: Duration = Duration::from_millis(100);
const TELEM_INTERVAL: Duration = Duration::from_secs(1);
const STREAM_STACK_SIZE: usize = 8192;
const FRAME_BUF_HINT: usize = 16 * 1024;

const XCLK_LEDC_TIMER: i32 = 0;
const OV3660_XCLK_MHZ: i32 = 16;
const OV3660_BRIGHTNESS: i32 = 1;
const OV3660_SATURATION: i32 = -2;

#[derive(PartialEq, Eq, Clone, Copy)]
enum SensorModel {
    Ov2640,
    Ov3660,
    Unknown,
}

impl SensorModel {
    fn detect() -> Self {
        let sensor = unsafe { cam::esp_camera_sensor_get() };
        if sensor.is_null() {
            return Self::Unknown;
        }
        let pid = unsafe { (*sensor).id.PID as u32 };
        match pid {
            cam::camera_pid_t_OV2640_PID => Self::Ov2640,
            cam::camera_pid_t_OV3660_PID => Self::Ov3660,
            _ => Self::Unknown,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Ov2640 => "ov2640",
            Self::Ov3660 => "ov3660",
            Self::Unknown => "unknown",
        }
    }
}

pub fn spawn(frames: SyncSender<Vec<u8>>) -> Result<()> {
    tasks::configure(c"cam", Priority::Camera, Core::Core1)?;
    Builder::new()
        .stack_size(STREAM_STACK_SIZE)
        .spawn(move || {
            let cam = match init() {
                Ok(cam) => cam,
                Err(e) => {
                    log::error!("camera init: {e}");
                    return;
                }
            };
            let model = SensorModel::detect();
            if model == SensorModel::Ov3660 {
                if let Err(e) = tune_ov3660(&cam) {
                    log::error!("ov3660 tune: {e}");
                    return;
                }
            }
            log::info!("camera ready ({} qvga jpeg, psram fb)", model.name());
            if let Err(e) = stream_loop(&cam, frames) {
                log::error!("camera stream stopped: {e}");
            }
        })
        .context("spawn camera thread")?;
    Ok(())
}

fn init() -> Result<Camera<'static>> {
    // SAFETY: camera pins are stolen once here and never taken from Peripherals elsewhere.
    let camera = unsafe {
        Camera::new(
            None,
            None,
            AnyIOPin::steal(CAM_PIN_XCLK),
            AnyIOPin::steal(CAM_PIN_D0),
            AnyIOPin::steal(CAM_PIN_D1),
            AnyIOPin::steal(CAM_PIN_D2),
            AnyIOPin::steal(CAM_PIN_D3),
            AnyIOPin::steal(CAM_PIN_D4),
            AnyIOPin::steal(CAM_PIN_D5),
            AnyIOPin::steal(CAM_PIN_D6),
            AnyIOPin::steal(CAM_PIN_D7),
            AnyIOPin::steal(CAM_PIN_VSYNC),
            AnyIOPin::steal(CAM_PIN_HREF),
            AnyIOPin::steal(CAM_PIN_PCLK),
            AnyIOPin::steal(CAM_PIN_SIOD),
            AnyIOPin::steal(CAM_PIN_SIOC),
            cam::pixformat_t_PIXFORMAT_JPEG,
            cam::framesize_t_FRAMESIZE_QVGA,
            cam::camera_grab_mode_t_CAMERA_GRAB_LATEST,
            cam::camera_fb_location_t_CAMERA_FB_IN_PSRAM,
        )
    };
    camera.context("esp_camera_init")
}

fn tune_ov3660(camera: &Camera<'static>) -> Result<()> {
    let Some(sensor) = camera.sensor() else {
        bail!("esp_camera_sensor_get returned null");
    };
    // esp_cam hardcodes 20MHz XCLK (OV2640 default); OV3660 JPEG output is
    // unstable at 20MHz on this driver (frames rejected as NO-SOI). 16MHz is
    // the tuned clock. Framesize must be reapplied so the sensor PLL is
    // recomputed from the new xclk_freq_hz.
    sensor
        .set_xclk(XCLK_LEDC_TIMER, OV3660_XCLK_MHZ)
        .context("set xclk 16MHz")?;
    sensor
        .set_framesize(cam::framesize_t_FRAMESIZE_QVGA)
        .context("reapply framesize")?;
    sensor.set_vflip(true).context("set vflip")?;
    sensor
        .set_brightness(OV3660_BRIGHTNESS)
        .context("set brightness")?;
    sensor
        .set_saturation(OV3660_SATURATION)
        .context("set saturation")?;
    Ok(())
}

fn stream_loop(cam: &Camera<'static>, frames: SyncSender<Vec<u8>>) -> Result<()> {
    let mut last_telem: Option<Instant> = None;
    loop {
        send_frame(cam, &frames)?;
        let now = Instant::now();
        let due = match last_telem {
            Some(t) => now - t >= TELEM_INTERVAL,
            None => true,
        };
        if due {
            let mut buf = Vec::with_capacity(64);
            let uptime_s = (unsafe { sys::esp_timer_get_time() }.max(0) / 1_000_000) as u64;
            proto::encode_telemetry(unsafe { sys::esp_get_free_heap_size() }, uptime_s, &mut buf);
            let _ = frames.send(buf);
            last_telem = Some(now);
        }
        std::thread::sleep(FRAME_INTERVAL);
    }
}

fn send_frame(cam: &Camera<'static>, frames: &SyncSender<Vec<u8>>) -> Result<()> {
    let Some(fb) = cam.get_framebuffer() else {
        bail!("esp_camera_fb_get returned null");
    };
    let mut out = Vec::with_capacity(FRAME_BUF_HINT);
    proto::encode_cam_frame(fb.data(), &mut out);
    let _ = frames.send(out);
    Ok(())
}
