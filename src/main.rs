use std::{time::Duration, str::FromStr};

use clap::{Parser, Subcommand, Args};
use tokio_serial::{available_ports, SerialPortBuilderExt};
use parse_duration::parse::{parse as parse_dur, Error as ParseDurationError};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Lists connected serial devices
    List,
    /// Gets readings from spectrometer
    Read(ReadingConf),
}

struct ParsableDuration(Duration);

impl FromStr for ParsableDuration {
    type Err = ParseDurationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_dur(s).map(|d| ParsableDuration(d))
    }
}

#[derive(Args)]
struct ReadingConf {
    /// Name of serial port that should be used
    serial: String,
    /// How long to scan for
    duration: ParsableDuration,
    /// Serial port baud rate
    baud_rate: u32
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::List => list_serial().expect("Getting list of serial ports failed"),
        Commands::Read(conf) => get_readings(conf).await.expect("Reading from serial failed"),
    }
}

fn list_serial() -> tokio_serial::Result<()> {
    let ports = available_ports()?;
    ports.iter().for_each(|port| println!("{}", port.port_name));

    Ok(())
}

async fn get_readings(conf: &ReadingConf) -> tokio_serial::Result<()> {
    let mut port = tokio_serial::new(conf.serial.clone(), conf.baud_rate).open_native_async()?;
    /*
    #[cfg(unix)]
    port.set_exclusive(false)?;
    */

    Ok(())
}
