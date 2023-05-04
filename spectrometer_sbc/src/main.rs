use std::{error::Error, thread, time::Duration};

use log::{info, debug};
use rppal::gpio::{Gpio, Level, Trigger};
use ccd_lcamv06::{StdIoAdapter, IoAdapter};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // safety: parsed as u8 during compilation time
    let laser_pin: u8 = std::env!("LASER_PIN").parse().unwrap();
    let button_pin: u8 = std::env!("BUTTON_PIN").parse().unwrap();

    let gpio = Gpio::new()?;

    let mut laser = gpio.get(laser_pin)?.into_output();
    laser.set_high();
    let mut button = gpio.get(button_pin)?.into_input_pullup();
    button.set_interrupt(Trigger::FallingEdge)?;
    debug!("Initialized GPIO");

    let mut serial = serialport::new("/dev/ttyUSB0", 115200).timeout(Duration::from_millis(100)).open()?;
    let ccd = StdIoAdapter::new(&mut serial).open_ccd();
    debug!("Initialized CCD");

    info!("Initialization complete");
    loop {
        button.poll_interrupt(false, None)?;
        laser.set_low();
        thread::sleep(Duration::from_millis(10));
        laser.set_high();
    }
}
