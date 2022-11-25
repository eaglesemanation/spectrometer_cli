mod cli;

use clap::Parser;
use futures::{SinkExt, StreamExt};
use num_traits::ToPrimitive;
use simple_eyre::{eyre::eyre, Result};
use std::{io::Write, path::Path};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use tokio::{
    fs::{read_to_string, File},
    io::AsyncWriteExt,
    time::{sleep, Duration},
};
use tokio_serial::{available_ports, SerialPortInfo};

use ccd_lcamv06::{
    decode_from_string, handle_ccd_response, try_new_ccd, BaudRate, Command as CCDCommand,
    Response as CCDResponse,
};
use cli::*;

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
            ReadCommands::HexFile(conf) => get_hex_file_reading(conf).await,
        },
        Commands::BaudRate(subcomm) => match &subcomm.command {
            BaudRateCommands::Get(conf) => get_baud_rate(conf).await,
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

/// Returns std::io::Write stream with coloring enabled if program is run interactively
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
    }
    stdout.reset()?;
    paths.iter().for_each(|p| println!("{}", p));

    Ok(())
}

fn frame_to_hex(frame: &ccd_lcamv06::Frame) -> String {
    frame
        // Split frame into 4 word lines
        .chunks(4)
        .map(|chunk| {
            chunk
                .iter()
                .map(|pixel| {
                    // Format each pixel as 4 letter hex word
                    let [b1, b2] = u16::to_be_bytes(*pixel);
                    return format!("{:02X}{:02X}", b1, b2);
                })
                .collect::<Vec<_>>()
                // Separate each work with a space
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn frames_to_hex(frames: &[ccd_lcamv06::Frame]) -> String {
    frames
        .iter()
        .map(frame_to_hex)
        .collect::<Vec<_>>()
        // Separate each frame by 2 empty lines
        .join("\n\n\n")
}

fn frame_to_csv(frame: &ccd_lcamv06::Frame) -> String {
    frame
        .iter()
        .map(|pixel| pixel.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn frames_to_csv(frames: &[ccd_lcamv06::Frame]) -> String {
    frames
        .iter()
        .map(frame_to_csv)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Gets readings for specified duration of time
async fn get_duration_reading(conf: &DurationReadingConf) -> Result<()> {
    let mut frames: Vec<_> = Vec::new();
    let timeout = sleep(Duration::from_secs(conf.duration.into()));
    tokio::pin!(timeout);

    let mut ccd = try_new_ccd(&(&conf.reading.serial).into()).await?;

    ccd.send(CCDCommand::ContinuousRead).await?;
    loop {
        tokio::select! {
            resp = ccd.next() => {
                if let Err(e) = handle_ccd_response!(
                    resp, CCDResponse::SingleReading,
                    |frame: ccd_lcamv06::Frame| {frames.push(frame); return Ok(())}
                ) {
                    eprintln!("Skipped frame: {}", e);
                    continue;
                };
            },
            _ = &mut timeout => {
                break;
            }
        }
    }
    ccd.send(CCDCommand::PauseRead).await?;

    match conf.reading.format {
        OutputFormat::CSV => {
            let mut out = File::create(&conf.reading.output).await?;
            out.write_all(frames_to_csv(&frames).as_bytes()).await?;
        }
        OutputFormat::Hex => {
            let mut out = File::create(&conf.reading.output).await?;
            out.write_all(frames_to_hex(&frames).as_bytes()).await?;
        }
    };

    Ok(())
}

async fn get_single_reading(conf: &SingleReadingConf) -> Result<()> {
    let mut ccd = try_new_ccd(&(&conf.serial).into()).await?;
    ccd.send(CCDCommand::SingleRead).await?;
    let frame = handle_ccd_response!(ccd.next().await, CCDResponse::SingleReading, |frame| Ok(
        frame
    ))?;

    match conf.format {
        OutputFormat::CSV => {
            let mut out = File::create(&conf.output).await?;
            out.write_all(frame_to_csv(&frame).as_bytes()).await?;
        }
        OutputFormat::Hex => {
            let mut out = File::create(&conf.output).await?;
            out.write_all(frame_to_hex(&frame).as_bytes()).await?;
        }
    };

    Ok(())
}

async fn get_hex_file_reading(conf: &HexFileReadingConf) -> Result<()> {
    let input_str = read_to_string(&conf.input).await?;
    let responses = decode_from_string(&input_str);
    let frames: Vec<_> = responses
        .iter()
        .enumerate()
        .filter_map(|(i, resp)| match resp {
            Ok(CCDResponse::SingleReading(frame)) => Some(frame.clone()),
            Ok(_) => None,
            Err(err) => {
                println!("Couldn't parse package #{}: {}", i + 1, err);
                None
            }
        })
        .collect();

    match conf.format {
        OutputFormat::CSV => {
            let mut out = File::create(&conf.output).await?;
            out.write_all(frames_to_csv(&frames).as_bytes()).await?;
        }
        OutputFormat::Hex => {
            let mut out = File::create(&conf.output).await?;
            out.write_all(frames_to_hex(&frames).as_bytes()).await?;
        }
    };

    Ok(())
}

async fn get_version(conf: &SerialConf) -> Result<()> {
    let mut ccd = try_new_ccd(&conf.into()).await?;
    ccd.send(CCDCommand::GetVersion).await?;
    handle_ccd_response!(
        ccd.next().await,
        CCDResponse::VersionInfo,
        |info: ccd_lcamv06::VersionDetails| {
            println!("{}", info);
            Ok(())
        }
    )?;
    Ok(())
}

async fn get_baud_rate(conf: &SerialConf) -> Result<()> {
    let mut ccd = try_new_ccd(&conf.into()).await?;
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

async fn get_avg_time(conf: &SerialConf) -> Result<()> {
    let mut ccd = try_new_ccd(&conf.into()).await?;
    ccd.send(CCDCommand::GetAverageTime).await?;
    handle_ccd_response!(ccd.next().await, CCDResponse::AverageTime, |avg_t: u8| {
        println!("Current \"average time\": {}", avg_t);
        Ok(())
    })?;
    Ok(())
}

async fn set_avg_time(conf: &SetAvgTimeConf) -> Result<()> {
    let mut ccd = try_new_ccd(&(&conf.serial).into()).await?;
    ccd.send(CCDCommand::SetAverageTime(conf.average_time))
        .await?;
    Ok(())
}

async fn get_exp_time(conf: &SerialConf) -> Result<()> {
    let mut ccd = try_new_ccd(&conf.into()).await?;
    ccd.send(CCDCommand::GetExposureTime).await?;
    handle_ccd_response!(ccd.next().await, CCDResponse::ExposureTime, |exp_t: u16| {
        println!("Current \"exposure time\": {}", exp_t);
        Ok(())
    })?;
    Ok(())
}

async fn set_exp_time(conf: &SetExpTimeConf) -> Result<()> {
    let mut ccd = try_new_ccd(&(&conf.serial).into()).await?;
    ccd.send(CCDCommand::SetIntegrationTime(conf.exposure_time))
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_frame_to_hex() {
        let frame: ccd_lcamv06::Frame = [u16::from_be_bytes([0xA1, 0xB2]); ccd_lcamv06::FRAME_SIZE];
        let hex = frame_to_hex(&frame);
        let hex_lines: Vec<_> = hex.split("\n").collect();
        assert_eq!(hex_lines[0], "A1B2 A1B2 A1B2 A1B2");
    }

    #[test]
    fn convert_frame_to_csv() {
        let frame: ccd_lcamv06::Frame = [1000; ccd_lcamv06::FRAME_SIZE];
        let csv = frame_to_csv(&frame);
        let csv_fields: Vec<_> = csv.split(",").collect();
        assert_eq!(csv_fields[0], "1000");
    }
}
