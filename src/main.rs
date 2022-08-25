#[allow(dead_code)]
mod ccd_codec;

use clap::{Args, Parser, Subcommand};
use futures::{sink::SinkExt, stream::StreamExt};
use std::{io, time::Duration};
use tokio::time::sleep;
use tokio_serial::{available_ports, SerialPortBuilderExt};
use tokio_util::codec::Decoder;

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

#[derive(Args)]
struct ReadingConf {
    /// Name of serial port that should be used
    serial: String,
    /// Duration for which frames are continiously captured
    duration: u8,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::List => list_serial().expect("Getting list of serial ports failed"),
        Commands::Read(conf) => get_readings(conf)
            .await
            .expect("Reading from serial failed"),
    }
}

fn list_serial() -> Result<(), io::Error> {
    let ports = available_ports()?;
    ports.iter().for_each(|port| println!("{}", port.port_name));

    Ok(())
}

async fn get_readings(conf: &ReadingConf) -> Result<(), io::Error> {
    let mut frames: Vec<_> = Vec::new();
    let timeout = sleep(Duration::from_secs(conf.duration.into()));
    tokio::pin!(timeout);

    // TODO: Dynamically change baudrate
    let port = tokio_serial::new(conf.serial.clone(), 115200).open_native_async()?;
    /*
    #[cfg(unix)]
    port.set_exclusive(false)?;
    */

    let mut ccd = ccd_codec::CCDCodec.framed(port);
    ccd.send(ccd_codec::Command::ContinuousRead).await?;
    loop {
        tokio::select! {
            resp = ccd.next() => {
                match resp {
                    Some(Ok(ccd_codec::Response::SingleReading(frame))) => {
                        frames.push(frame);
                    }
                    Some(Ok(_)) => {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "Got unexpected response while waiting for readings"));
                    }
                    Some(Err(err)) => {
                        return Err(err);
                    }
                    None => {}
                }
            },
            _ = &mut timeout => {
                break;
            }
        }
    }
    ccd.send(ccd_codec::Command::PauseRead).await?;

    Ok(())
}
