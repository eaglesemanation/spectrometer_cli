fn main() {
    match dotenvy::dotenv() {
        Ok(_) => {},
        Err(err) => {
            if !err.not_found() {
                panic!("{}", err)
            }
        }
    }

    let laser_pin: u8 = std::env::var("LASER_PIN")
        .expect("LASER_PIN env var is required")
        .parse()
        .expect("LASER_PIN needs to be an integer");
    let button_pin: u8 = std::env::var("TRIGGER_PIN")
        .expect("TRIGGER_PIN env var is required")
        .parse()
        .expect("TRIGGER_PIN needs to be an integer");

    if laser_pin == button_pin {
        panic!("LASER_PIN and TRIGGER_PIN need to be different");
    }

    println!("cargo:rustc-env=LASER_PIN={}", laser_pin);
    println!("cargo:rustc-env=TRIGGER_PIN={}", button_pin);
}
