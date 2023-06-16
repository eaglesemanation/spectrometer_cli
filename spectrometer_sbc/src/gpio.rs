use std::sync::{Arc, Mutex, OnceLock};
use log::info;
use rppal::gpio::{OutputPin, Gpio};

pub struct Pins {
    pub laser_pin: Mutex<OutputPin>
}

pub fn get_pins() -> Result<&'static Arc<Pins>, Box<dyn std::error::Error>> {
    static PINS: OnceLock<Arc<Pins>> = OnceLock::new();
    PINS.get_or_try_init(|| {
        info!("Initializing GPIO");

        let laser_pin_num: u8 = env!("LASER_PIN").parse()?;

        let gpio = Gpio::new()?;
        
        Ok(Arc::new(Pins{
            laser_pin: Mutex::new(gpio.get(laser_pin_num)?.into_output())
        }))
    })
}
