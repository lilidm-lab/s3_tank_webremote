use std::sync::mpsc::Receiver;
use std::thread::Builder;

use anyhow::{Context, Result};
use esp_idf_svc::hal::cpu::Core;
use esp_idf_svc::hal::gpio::{Output, OutputPin, PinDriver, Pins};
use esp_idf_svc::hal::ledc::{
    config::TimerConfig, LedcDriver, LedcTimerDriver, LowSpeed, Resolution, LEDC,
};
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::sys::EspError;

use crate::proto::{Direction, Track};
use crate::tasks::{self, Priority};

const PWM_FREQ_HZ: u32 = 20_000;
const PWM_RESOLUTION: Resolution = Resolution::Bits10;
const DRIVE_DUTY_PERMILLE: u32 = 700;
const MOTOR_STACK_SIZE: usize = 4096;

pub struct Tank {
    left: Motor,
    right: Motor,
    duty: u32,
}

struct Motor {
    in1: PinDriver<'static, Output>,
    in2: PinDriver<'static, Output>,
    pwm: LedcDriver<'static>,
}

impl Tank {
    pub fn new(pins: Pins, ledc: LEDC) -> Result<Self> {
        let timer = LedcTimerDriver::new(
            ledc.timer1,
            &TimerConfig::new()
                .frequency(Hertz(PWM_FREQ_HZ))
                .resolution(PWM_RESOLUTION),
        )?;
        let left = Motor::new(pins.gpio1, pins.gpio2, pins.gpio42, ledc.channel2, &timer)?;
        let right = Motor::new(pins.gpio41, pins.gpio40, pins.gpio39, ledc.channel3, &timer)?;
        let duty = left.pwm.get_max_duty() * DRIVE_DUTY_PERMILLE / 1000;
        Ok(Self { left, right, duty })
    }

    pub fn drive(&mut self, dir: Direction) -> std::result::Result<(), EspError> {
        let (l, r) = dir.tracks();
        self.left.drive(l, self.duty)?;
        self.right.drive(r, self.duty)
    }
}

impl Motor {
    fn new(
        in1: impl OutputPin + 'static,
        in2: impl OutputPin + 'static,
        pwm_pin: impl OutputPin + 'static,
        channel: impl esp_idf_svc::hal::ledc::LedcChannel<SpeedMode = LowSpeed> + 'static,
        timer: &LedcTimerDriver<'static, LowSpeed>,
    ) -> Result<Self, EspError> {
        Ok(Self {
            in1: PinDriver::output(in1)?,
            in2: PinDriver::output(in2)?,
            pwm: LedcDriver::new(channel, timer, pwm_pin)?,
        })
    }

    fn drive(&mut self, track: Track, duty: u32) -> std::result::Result<(), EspError> {
        match track {
            Track::Forward => {
                self.in1.set_high()?;
                self.in2.set_low()?;
            }
            Track::Reverse => {
                self.in1.set_low()?;
                self.in2.set_high()?;
            }
            Track::Halt => {
                self.in1.set_low()?;
                self.in2.set_low()?;
            }
        }
        let duty = if track == Track::Halt { 0 } else { duty };
        self.pwm.set_duty(duty)
    }
}

pub fn spawn(tank: Tank, cmds: Receiver<Direction>) -> Result<()> {
    tasks::configure(c"motor", Priority::Motor, Core::Core1)?;
    Builder::new()
        .stack_size(MOTOR_STACK_SIZE)
        .spawn(move || {
            let mut tank = tank;
            while let Ok(dir) = cmds.recv() {
                if let Err(e) = tank.drive(dir) {
                    log::error!("drive {dir:?}: {e}");
                }
            }
        })
        .context("spawn motor thread")?;
    Ok(())
}
