use log::info;
use rppal::gpio::{Gpio, OutputPin, Level};
use tokio::{sync::watch::{self, Receiver}, task::JoinHandle};
use anyhow::bail;

pub struct Pins {
    pub laser_pin: OutputPin,
    pub trigger_worker: JoinHandle<anyhow::Result<()>>,
    pub trigger_state: Receiver<Level>
}

impl Pins {
    pub fn init() -> anyhow::Result<Pins> {
        info!("Initializing GPIO");

        let laser_pin_num: u8 = env!("LASER_PIN").parse()?;
        let trigger_pin_num: u8 = env!("TRIGGER_PIN").parse()?;

        let gpio = Gpio::new()?;
        let laser_pin = gpio.get(laser_pin_num)?.into_output();
        let mut trigger_pin = gpio.get(trigger_pin_num)?.into_input_pullup();
        trigger_pin.set_interrupt(rppal::gpio::Trigger::Both)?;

        let (tx, rx) = watch::channel(trigger_pin.read());

        let trigger_worker =
            tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
                loop {
                    match trigger_pin.poll_interrupt(true, None)? {
                        Some(lvl) => {
                            tx.send_if_modified(|state| {
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

        Ok(Pins {
            laser_pin,
            trigger_worker,
            trigger_state: rx
        })
    }

}
