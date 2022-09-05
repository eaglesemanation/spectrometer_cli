#[allow(dead_code)]
mod ccd_codec;

use clap::{Args, Parser, Subcommand};
use futures::{sink::SinkExt, stream::StreamExt};
use num_traits::{FromPrimitive, ToPrimitive};
use simple_eyre::{eyre::eyre, Result};
use std::{io::Write, path::Path, time::Duration};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use tokio::{fs::File, io::AsyncWriteExt, time::sleep};
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

#[derive(Args)]
struct SerialConf {
    /// Name of serial port that should be used
    #[clap(short, long, value_parser)]
    serial: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Lists connected serial devices
    List,
    /// Get version info from CCD
    CCDVersion(SerialConf),
    /// Get readings from spectrometer
    Read(ReadCommand),
    /// Baud rate related commands
    BaudRate(BaudRateCommand),
    /// "Average time" related commands, not sure what that really means
    AverageTime(AvgTimeCommand),
    /// "Exposure time" related commands, not sure how that's different from "averate time"
    ExposureTime(ExpTimeCommand),
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

#[derive(Args)]
struct BaudRateCommand {
    #[clap(subcommand)]
    command: BaudRateCommands,
}

#[derive(Subcommand)]
enum BaudRateCommands {
    /// Get current baud rate
    Get(SerialConf),
    /// Set baud rate
    Set(SetBaudRateConf),
}

#[derive(Args)]
struct SetBaudRateConf {
    /// New baud rate
    #[clap(value_parser = parse_baud_rate)]
    baud_rate: BaudRate,

    #[clap(flatten)]
    serial: SerialConf,
}

fn parse_baud_rate(s: &str) -> Result<BaudRate> {
    let n: u32 = s.parse()?;
    BaudRate::from_u32(n).ok_or(eyre!(
        "Baud rate of {} is not supported, use one of these: {:?}",
        n,
        BaudRate::all_baud_rates()
            .iter()
            .map(|b| b.to_u32().unwrap())
            .collect::<Vec<u32>>()
    ))
}

#[derive(Args)]
struct AvgTimeCommand {
    #[clap(subcommand)]
    command: AvgTimeCommands,
}

#[derive(Subcommand)]
enum AvgTimeCommands {
    /// Get current "average time"
    Get(SerialConf),
    /// Set "average time"
    Set(SetAvgTimeConf),
}

#[derive(Args)]
struct SetAvgTimeConf {
    /// New "average time"
    #[clap(value_parser)]
    average_time: u8,
    #[clap(flatten)]
    serial: SerialConf,
}

#[derive(Args)]
struct ExpTimeCommand {
    #[clap(subcommand)]
    command: ExpTimeCommands,
}

#[derive(Subcommand)]
enum ExpTimeCommands {
    /// Get current "exposure time"
    Get(SerialConf),
    /// Set "exposure time"
    Set(SetExpTimeConf),
}

#[derive(Args)]
struct SetExpTimeConf {
    /// New "exposure time"
    #[clap(value_parser)]
    exposure_time: u16,
    #[clap(flatten)]
    serial: SerialConf,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    simple_eyre::install()?;
    let cli = Cli::parse();

    match &cli.command {
        Commands::List => list_serial(),
        Commands::CCDVersion(conf) => get_version(conf).await,
        Commands::Read(subcomm) => match &subcomm.command {
            ReadCommands::Single(conf) => get_single_reading(conf).await,
            ReadCommands::Duration(conf) => get_duration_reading(conf).await,
        },
        Commands::BaudRate(subcomm) => match &subcomm.command {
            BaudRateCommands::Get(conf) => get_baud_rate(conf).await,
            BaudRateCommands::Set(conf) => set_baud_rate(conf).await,
        },
        Commands::AverageTime(subcomm) => match &subcomm.command {
            AvgTimeCommands::Get(conf) => get_avg_time(conf).await,
            AvgTimeCommands::Set(conf) => set_avg_time(conf).await,
        },
        Commands::ExposureTime(subcomm) => match &subcomm.command {
            ExpTimeCommands::Get(conf) => get_exp_time(conf).await,
            ExpTimeCommands::Set(conf) => set_exp_time(conf).await,
        },
    }
}

/// Returns std::io::Write stream with coloring enabled if programm is run interactively
fn get_stdout() -> StandardStream {
    StandardStream::stdout(if atty::is(atty::Stream::Stdout) {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    })
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
    ports.iter().map(port_to_path).collect()
}

fn list_serial() -> Result<()> {
    let mut stdout = get_stdout();
    let paths = get_port_paths()?;
    if paths.is_empty() {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
        writeln!(&mut stdout, "No connected serial ports found.")?;
    } else {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        writeln!(&mut stdout, "Connected serial ports:")?;
        stdout.reset()?;
        paths.iter().for_each(|p| println!("{}", p));
    }

    Ok(())
}

async fn try_new_ccd(conf: &SerialConf) -> Result<Framed<SerialStream, CCDCodec>> {
    // Use default baud rate for initial connection
    let port = tokio_serial::new(conf.serial.clone(), BaudRate::default().to_u32().unwrap())
        .open_native_async()?;
    let mut ccd = ccd_codec::CCDCodec.framed(port);
    ccd.send(CCDCommand::GetSerialBaudRate).await?;

    // Try to change to currently configured baud rate
    let current_baud_rate = handle_ccd_response!(
        ccd.next().await,
        CCDResponse::SerialBaudRate,
        |b: BaudRate| { Ok(b) }
    )?;
    if current_baud_rate != BaudRate::default() {
        drop(ccd);
        let port = tokio_serial::new(conf.serial.clone(), current_baud_rate.to_u32().unwrap())
            .open_native_async()?;
        ccd = ccd_codec::CCDCodec.framed(port);
    }

    Ok(ccd)
}

/// Gets readings for specified duration of time
async fn get_duration_reading(conf: &DurationReadingConf) -> Result<()> {
    let mut frames: Vec<_> = Vec::new();
    let timeout = sleep(Duration::from_secs(conf.duration.into()));
    tokio::pin!(timeout);

    let mut ccd = try_new_ccd(&conf.reading.serial).await?;

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
    let mut ccd = try_new_ccd(&conf.serial).await?;
    ccd.send(CCDCommand::SingleRead).await?;
    let frame = handle_ccd_response!(ccd.next().await, CCDResponse::SingleReading, |frame| Ok(
        frame
    ))?;

    let mut out = File::create(&conf.output).await?;
    out.write_all(format!("{:#?}", frame).as_bytes()).await?;

    Ok(())
}

async fn get_version(conf: &SerialConf) -> Result<()> {
    let mut ccd = try_new_ccd(&conf).await?;
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
    let mut ccd = try_new_ccd(&conf).await?;
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

async fn set_baud_rate(conf: &SetBaudRateConf) -> Result<()> {
    let mut ccd = try_new_ccd(&conf.serial).await?;
    ccd.send(CCDCommand::SetSerialBaudRate(conf.baud_rate))
        .await?;
    Ok(())
}

async fn get_avg_time(conf: &SerialConf) -> Result<()> {
    let mut ccd = try_new_ccd(&conf).await?;
    ccd.send(CCDCommand::GetAverageTime).await?;
    handle_ccd_response!(ccd.next().await, CCDResponse::AverageTime, |avg_t: u8| {
        println!("Current \"average time\": {}", avg_t);
        Ok(())
    })?;
    Ok(())
}

async fn set_avg_time(conf: &SetAvgTimeConf) -> Result<()> {
    let mut ccd = try_new_ccd(&conf.serial).await?;
    ccd.send(CCDCommand::SetAverageTime(conf.average_time))
        .await?;
    Ok(())
}

async fn get_exp_time(conf: &SerialConf) -> Result<()> {
    let mut ccd = try_new_ccd(&conf).await?;
    ccd.send(CCDCommand::GetExposureTime).await?;
    handle_ccd_response!(ccd.next().await, CCDResponse::ExposureTime, |exp_t: u16| {
        println!("Current \"exposure time\": {}", exp_t);
        Ok(())
    })?;
    Ok(())
}

async fn set_exp_time(conf: &SetExpTimeConf) -> Result<()> {
    let mut ccd = try_new_ccd(&conf.serial).await?;
    ccd.send(CCDCommand::SetIntegrationTime(conf.exposure_time))
        .await?;
    Ok(())
}
