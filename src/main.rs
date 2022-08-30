#[allow(dead_code)]
mod ccd_codec;

use clap::{Args, Parser, Subcommand};
use color_eyre::{
    eyre::{eyre, WrapErr},
    Result,
};
use colored::*;
use futures::{sink::SinkExt, stream::StreamExt};
use std::{path::Path, time::Duration};
use tokio::time::sleep;
use tokio_serial::{available_ports, SerialPortBuilderExt, SerialPortInfo};
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
    #[clap(short, long, value_parser)]
    serial: String,
    /// Duration in seconds for which frames are continiously captured
    #[clap(short, long, value_parser, default_value = "3")]
    duration: u8,
    /// Path to a file where readings should be stored
    #[clap(short, long, value_parser, value_hint = clap::ValueHint::FilePath)]
    output: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::List => list_serial(),
        Commands::Read(conf) => get_readings(conf).await,
    }
}

#[cfg(target_os = "linux")]
fn port_to_path(port: &SerialPortInfo) -> Result<String> {
    let dev_path = port
        .port_name
        .split('/')
        .last()
        .map(|d| format!("/dev/{}", d))
        .ok_or(eyre!("Could not map /sys/class/tty to /dev"))?;
    if Path::new(dev_path.as_str()).exists() {
        Ok(dev_path)
    } else {
        // It's quite possibe that udev can rename tty devices while mapping from /sys to /dev, but
        // I just don't want to link against libudev, this is supposed to be a small static project
        Err(eyre!(
            "Could not find port {}, even though {} exists",
            dev_path,
            port.port_name
        ))
    }
}

#[cfg(not(target_os = "linux"))]
fn port_to_path(port: &SerialPortInfo) -> Result<String> {
    Ok(port.port_name.clone())
}

fn get_port_paths() -> Result<Vec<String>> {
    let ports = available_ports()?;
    ports
        .iter()
        .map(port_to_path)
        .filter(|path_res| match path_res {
            Ok(path) => !path.is_empty(),
            Err(_) => false
        })
        .collect()
}

fn list_serial() -> Result<()> {
    let paths = get_port_paths()?;
    if paths.is_empty() {
        println!("{}", "No connected serial ports found.".red())
    } else {
        println!("{}", "Connected serial ports:".green());
        paths.iter().for_each(|p| println!("{}", p));
    }

    Ok(())
}

async fn get_readings(conf: &ReadingConf) -> Result<()> {
    let mut frames: Vec<_> = Vec::new();
    let timeout = sleep(Duration::from_secs(conf.duration.into()));
    tokio::pin!(timeout);

    // TODO: Dynamically change baudrate
    let mut port = tokio_serial::new(conf.serial.clone(), 115200).open_native_async()?;

    #[cfg(unix)]
    port.set_exclusive(false)?;

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
                        return Err(eyre!("Got unexpected response while waiting for readings"));
                    }
                    Some(err) => {
                        return err.map(|_| ()).wrap_err("Unexpected end of serial port stream");
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
