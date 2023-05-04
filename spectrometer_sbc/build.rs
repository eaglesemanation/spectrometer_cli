fn main() {
    let laser_pin: u8 = std::env::var("LASER_PIN")
        .expect("LASER_PIN env var is required")
        .parse()
        .expect("LASER_PIN needs to be an integer");
    let button_pin: u8 = std::env::var("BUTTON_PIN")
        .expect("BUTTON_PIN env var is required")
        .parse()
        .expect("BUTTON_PIN needs to be an integer");

    if laser_pin == button_pin {
        panic!("LASER_PIN and BUTTON_PIN need to be different");
    }

    println!("cargo:rustc-env=LASER_PIN={}", laser_pin);
    println!("cargo:rustc-env=BUTTON_PIN={}", button_pin);
}
