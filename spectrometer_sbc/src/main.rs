use std::error::Error;

use rppal::gpio::Gpio;

fn main() -> Result<(), Box<dyn Error>> {
    let gpio = Gpio::new()?;

    let mut led = gpio.get(23)?.into_output();
    let mut button = gpio.get(21)?.into_input_pullup();

    button.set_interrupt(rppal::gpio::Trigger::RisingEdge)?;

    for _ in 0..10 {
        button.poll_interrupt(false, None)?;
        led.toggle();
    }

    Ok(())
}
