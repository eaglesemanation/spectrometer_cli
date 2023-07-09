use anyhow::bail;
use log::info;
use rppal::gpio::{Gpio, Level, OutputPin};
use std::sync::{Arc, Mutex};
use tokio::{sync::watch, task::JoinHandle};

#[derive(Debug, Clone)]
pub struct GpioState {
    pub laser: Arc<Mutex<OutputPin>>,
    pub trigger: watch::Receiver<Level>,
}

pub struct GpioWorkers {
    pub trigger_worker: JoinHandle<anyhow::Result<()>>,
}

impl GpioState {
    pub fn init() -> anyhow::Result<(GpioState, GpioWorkers)> {
        info!("Initializing GPIO");

        let laser_pin_num: u8 = env!("LASER_PIN").parse()?;
        let trigger_pin_num: u8 = env!("TRIGGER_PIN").parse()?;

        let gpio = Gpio::new()?;
        let laser_pin = gpio.get(laser_pin_num)?.into_output();

        let mut trigger_pin = gpio.get(trigger_pin_num)?.into_input_pullup();
        trigger_pin.set_interrupt(rppal::gpio::Trigger::Both)?;
        let (trigger_tx, trigger_rx) = watch::channel(trigger_pin.read());
        let trigger_worker = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            loop {
                match trigger_pin.poll_interrupt(true, None)? {
                    Some(lvl) => {
                        trigger_tx.send_if_modified(|state| {
                            let changed = *state != lvl;
                            if changed {
                                info!("Trigger changed state")
                            }
                            *state = lvl;
                            changed
                        });
                    }
                    None => {
                        // Should not be reachable, timeout on poll_interupt set to None
                        bail!("Timed out");
                    }
                }
            }
        });

        Ok((
            GpioState {
                laser: Mutex::new(laser_pin).into(),
                trigger: trigger_rx,
            },
            GpioWorkers { trigger_worker },
        ))
    }
}
