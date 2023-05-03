use std::{error::Error, thread, time::Duration};

use rppal::gpio::{Gpio, Level};

fn main() -> Result<(), Box<dyn Error>> {
    let gpio = Gpio::new()?;

    let laser_pin: u8 = std::env!("LASER_PIN", "GPIO pin for laser is not defined").parse()?;
    let button_pin: u8 = std::env!("BUTTON_PIN", "GPIO pin for button is not defined").parse()?;

    let mut laser = gpio.get(laser_pin)?.into_output();
    let mut button = gpio.get(button_pin)?.into_input_pullup();

    button.set_interrupt(rppal::gpio::Trigger::FallingEdge)?;
    laser.set_high();

    let mut count_low = 0;

    loop {
        button.poll_interrupt(false, None)?;
        loop {
            match button.read() {
                Level::High => {
                    count_low = 0;
                    break;
                },
                Level::Low => count_low += 1,
            }
            if count_low > 10 {
                laser.set_low();
                thread::sleep(Duration::from_secs(3));
                laser.set_high();
                break
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}
