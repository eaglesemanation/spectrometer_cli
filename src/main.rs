#[allow(dead_code)]
mod ccd_codec;

use clap::{Args, Parser, Subcommand};
use color_eyre::{eyre::eyre, Result};
use colored::*;
use futures::{sink::SinkExt, stream::StreamExt};
use num_traits::{FromPrimitive, ToPrimitive};
use std::{path::Path, sync::Arc, time::Duration};
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    sync::{oneshot, Mutex as AsyncMutex},
    time::sleep,
};
use tokio_serial::{available_ports, SerialPortBuilderExt, SerialPortInfo, SerialStream};
use tokio_util::codec::{Decoder, Framed};

use ccd_codec::{
    handle_ccd_response, BaudRate, CCDCodec, Command as CCDCommand, Response as CCDResponse,
};

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
    /// Get current baud rate
    GetBaudRate(SerialConf),
    /// Get version info from CCD
    CCDVersion(SerialConf),
    /// Get readings from spectrometer
    Read(ReadCommand),
}

#[derive(Args)]
struct ReadCommand {
    #[clap(subcommand)]
    command: ReadCommands,
}

#[derive(Subcommand)]
enum ReadCommands {
    /// Get a single frame
    Single(SingleReadingConf),
    /// Continiously get readings for specified duration
    Duration(DurationReadingConf),
}

#[derive(Args)]
struct SerialConf {
    /// Name of serial port that should be used
    #[clap(short, long, value_parser)]
    serial: String,
    /// Baud rate
    #[clap(short, long, value_parser)]
    baud_rate: Option<u32>,
}

#[derive(Args)]
struct SingleReadingConf {
    /// Path to a file where readings should be stored
    #[clap(short, long, value_parser, value_hint = clap::ValueHint::FilePath)]
    output: String,

    #[clap(flatten)]
    serial: SerialConf,
}

#[derive(Args)]
struct DurationReadingConf {
    /// Duration in seconds for which frames are continiously captured
    #[clap(short, long, value_parser, default_value = "3")]
    duration: u8,

    #[clap(flatten)]
    reading: SingleReadingConf,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    match &cli.command {
        Commands::List => list_serial(),
        Commands::GetBaudRate(conf) => get_baud_rate(conf).await,
        Commands::CCDVersion(conf) => get_version(conf).await,
        Commands::Read(subcomm) => match &subcomm.command {
            ReadCommands::Single(conf) => get_single_reading(conf).await,
            ReadCommands::Duration(conf) => get_duration_reading(conf).await,
        },
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
            Err(_) => false,
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

struct CCDConn {
    ccd: Arc<AsyncMutex<Framed<SerialStream, CCDCodec>>>,
    drop_tx: Option<oneshot::Sender<()>>,
}

impl Drop for CCDConn {
    // Use channels because async drop is not a thing yet
    fn drop(&mut self) {
        if let Some(tx) = self.drop_tx.take() {
            tx.send(()).unwrap();
        }
    }
}

impl CCDConn {
    async fn try_new(conf: &SerialConf) -> Result<Self> {
        let (drop_tx, drop_rx) = oneshot::channel();

        // Initial connection for setting up baud rate
        let mut port =
            tokio_serial::new(conf.serial.clone(), BaudRate::default().to_u32().unwrap())
                .open_native_async()?;
        #[cfg(unix)]
        port.set_exclusive(false).unwrap();

        let mut ccd = ccd_codec::CCDCodec.framed(port);
        let conn = if conf.baud_rate.is_some()
            && conf.baud_rate.unwrap() != BaudRate::default().to_u32().unwrap()
        {
            // Change baud rate to a different one
            let baud_rate = BaudRate::from_u32(conf.baud_rate.unwrap()).unwrap();
            ccd.send(CCDCommand::SetSerialBaudRate(baud_rate)).await?;
            /*
            handle_ccd_response!(ccd.next().await, CCDResponse::SerialBaudRate, |b| {
                if b != baud_rate {
                    Err(io::Error::new(io::ErrorKind::InvalidData, "Incorrect response on baud rate"))
                } else {
                    Ok(())
                }
            })?;
            */
            drop(ccd);

            let mut port = tokio_serial::new(conf.serial.clone(), baud_rate.to_u32().unwrap())
                .open_native_async()?;
            #[cfg(unix)]
            port.set_exclusive(false).unwrap();
            let ccd = ccd_codec::CCDCodec.framed(port);
            let ccd = Arc::new(AsyncMutex::new(ccd));
            CCDConn {
                ccd,
                drop_tx: Some(drop_tx),
            }
        } else {
            // Use default baud rate
            let ccd = Arc::new(AsyncMutex::new(ccd));
            CCDConn {
                ccd,
                drop_tx: Some(drop_tx),
            }
        };

        let cloned_ccd = conn.ccd.clone();
        tokio::spawn(async move {
            drop_rx.await.unwrap();
            let mut ccd = cloned_ccd.lock().await;
            ccd.send(CCDCommand::SetSerialBaudRate(BaudRate::default().into()))
                .await
                .unwrap();
        });

        Ok(conn)
    }
}

/// Gets readings for specified duration of time
async fn get_duration_reading(conf: &DurationReadingConf) -> Result<()> {
    let mut frames: Vec<_> = Vec::new();
    let timeout = sleep(Duration::from_secs(conf.duration.into()));
    tokio::pin!(timeout);

    let ccd_conn = CCDConn::try_new(&conf.reading.serial).await?;
    let ccd = &mut ccd_conn.ccd.lock().await;

    ccd.send(CCDCommand::ContinuousRead).await?;
    loop {
        tokio::select! {
            resp = ccd.next() => {
                if let Err(err) = handle_ccd_response!(
                    resp, CCDResponse::SingleReading,
                    |frame: ccd_codec::Frame| {frames.push(frame); return Ok(())}
                ) {
                    println!("{:#}", err);
                    break;
                }
            },
            _ = &mut timeout => {
                break;
            }
        }
    }
    ccd.send(CCDCommand::PauseRead).await?;

    let mut out = File::create(&conf.reading.output).await?;
    out.write_all(format!("{:#?}", frames).as_bytes()).await?;

    Ok(())
}

async fn get_single_reading(conf: &SingleReadingConf) -> Result<()> {
    let ccd_conn = CCDConn::try_new(&conf.serial).await?;
    let ccd = &mut ccd_conn.ccd.lock().await;

    ccd.send(CCDCommand::SingleRead).await?;
    let frame = handle_ccd_response!(ccd.next().await, CCDResponse::SingleReading, |frame| Ok(
        frame
    ))?;

    let mut out = File::create(&conf.output).await?;
    out.write_all(format!("{:#?}", frame).as_bytes()).await?;

    Ok(())
}

async fn get_version(conf: &SerialConf) -> Result<()> {
    let ccd_conn = CCDConn::try_new(&conf).await?;
    let ccd = &mut ccd_conn.ccd.lock().await;

    ccd.send(CCDCommand::GetVersion).await?;
    handle_ccd_response!(
        ccd.next().await,
        CCDResponse::VersionInfo,
        |info: ccd_codec::VersionDetails| {
            println!("Hardware version: {}", info.hardware_version);
            println!("Firmware version: {}", info.firmware_version);
            println!("Sensor type: {}", info.sensor_type);
            println!("Serial number: {}", info.serial_number);
            Ok(())
        }
    )?;

    Ok(())
}

async fn get_baud_rate(conf: &SerialConf) -> Result<()> {
    let ccd_conn = CCDConn::try_new(&conf).await?;
    let ccd = &mut ccd_conn.ccd.lock().await;

    ccd.send(CCDCommand::GetSerialBaudRate).await?;
    handle_ccd_response!(
        ccd.next().await,
        CCDResponse::SerialBaudRate,
        |baud_rate: BaudRate| {
            let baud_rate = baud_rate.to_u32().unwrap();
            println!("Current baud rate: {}", baud_rate);
            Ok(())
        }
    )?;

    Ok(())
}
