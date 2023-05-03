use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rustc-env=LASER_PIN={}", std::env::var("LASER_PIN")?);
    println!("cargo:rustc-env=BUTTON_PIN={}", std::env::var("BUTTON_PIN")?);

    Ok(())
}
