mod cli;
mod output;
mod serial;

use ccd_lcamv06::FRAME_PIXEL_COUNT;
use clap::Parser;
use simple_eyre::Result;
use num_traits::ToPrimitive;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use cli::*;
use serial::SerialConf;

fn main() -> Result<()> {
    simple_eyre::install()?;
    let cli = Cli::parse();
    env_logger::init();

    match &cli.command {
        Commands::List => list_serial(),
        Commands::CCDVersion(conf) => get_version(conf),
        Commands::Read(subcomm) => match &subcomm.command {
            ReadCommands::Single(conf) => get_single_reading(conf),
            ReadCommands::Multi(conf) => get_multiple_readings(conf),
        },
        Commands::BaudRate(subcomm) => match &subcomm.command {
            BaudRateCommands::Get(conf) => get_baud_rate(conf),
            BaudRateCommands::Set(conf) => set_baud_rate(conf),
        },
        Commands::AverageTime(subcomm) => match &subcomm.command {
            AvgTimeCommands::Get(conf) => get_avg_time(conf),
            AvgTimeCommands::Set(conf) => set_avg_time(conf),
        },
        Commands::ExposureTime(subcomm) => match &subcomm.command {
            ExpTimeCommands::Get(conf) => get_exp_time(conf),
            ExpTimeCommands::Set(conf) => set_exp_time(conf),
        },
    }
}

/// Returns std::io::Write stream with coloring enabled if program is run interactively
fn get_stdout() -> StandardStream {
    StandardStream::stdout(if atty::is(atty::Stream::Stdout) {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    })
}

fn list_serial() -> Result<()> {
    let mut stdout = get_stdout();
    let paths = serialport::available_ports()?;
    if paths.is_empty() {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
        writeln!(&mut stdout, "No connected serial ports found.")?;
    } else {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        writeln!(&mut stdout, "Connected serial ports:")?;
    }
    stdout.reset()?;
    paths.iter().for_each(|p| println!("{}", p.port_name));

    Ok(())
}

fn get_multiple_readings(conf: &MultiReadingConf) -> Result<()> {
    let mut ccd = conf.serial.open_ccd()?;
    let mut frames: Vec<_> = Vec::with_capacity(conf.count);
    frames.resize(conf.count, [0; FRAME_PIXEL_COUNT]);

    ccd.get_frames(&mut frames)?;
    conf.output.write_frames(&frames)?;

    Ok(())
}

fn get_single_reading(conf: &SingleReadingConf) -> Result<()> {
    let mut ccd = conf.serial.open_ccd()?;
    let frame = ccd.get_frame()?;
    conf.output.write_frame(&frame)?;
    Ok(())
}

fn get_version(conf: &SerialConf) -> Result<()> {
    let mut ccd = conf.open_ccd()?;
    let version_details = ccd.get_version()?;
    println!("{version_details}");
    Ok(())
}

fn get_baud_rate(conf: &SerialConf) -> Result<()> {
    let mut ccd = conf.open_ccd()?;
    let baud_rate = ccd.get_baudrate()?.to_u32().unwrap();
    println!("Current baud rate: {baud_rate}");
    Ok(())
}

fn set_baud_rate(conf: &SetBaudRateConf) -> Result<()> {
    let mut ccd = conf.serial.open_ccd()?;
    ccd.set_baudrate(conf.baud_rate)?;
    Ok(())
}

fn get_avg_time(conf: &SerialConf) -> Result<()> {
    let mut ccd = conf.open_ccd()?;
    println!("Current \"average time\": {}", ccd.get_avg_time()?);
    Ok(())
}

fn set_avg_time(conf: &SetAvgTimeConf) -> Result<()> {
    let mut ccd = conf.serial.open_ccd()?;
    ccd.set_avg_time(conf.average_time)?;
    Ok(())
}

fn get_exp_time(conf: &SerialConf) -> Result<()> {
    let mut ccd = conf.open_ccd()?;
    println!("Current \"exposure time\": {}", ccd.get_exp_time()?);
    Ok(())
}

fn set_exp_time(conf: &SetExpTimeConf) -> Result<()> {
    let mut ccd = conf.serial.open_ccd()?;
    ccd.set_exp_time(conf.exposure_time)?;
    Ok(())
}
