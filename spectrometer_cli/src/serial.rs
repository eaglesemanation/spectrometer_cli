use ccd_lcamv06::{BaudRate, CCD};
use clap::Args;
use num_traits::ToPrimitive;
use serialport::SerialPort;
use simple_eyre::{eyre::eyre, Result};
use std::time::Duration;

#[derive(Args)]
pub struct SerialConf {
    /// Name of serial port that should be used
    #[clap(short, long, value_parser)]
    pub serial: String,
}

impl SerialConf {
    pub fn open_ccd(&self) -> Result<CCD<Box<dyn SerialPort>>> {
        let port = serialport::new(
            &self.serial,
            BaudRate::default().to_u32().unwrap(),
        )
        .timeout(Duration::from_millis(100))
        .open()
        .map_err(|_| eyre!("Could not open serial port"))?;
        Ok(CCD::new(port))
    }
}
