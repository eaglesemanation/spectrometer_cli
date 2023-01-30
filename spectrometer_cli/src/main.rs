mod cli;

use clap::Parser;
use num_traits::ToPrimitive;
use serialport::SerialPort;
use simple_eyre::{eyre::eyre, Result};
use std::{fs::File, io::Write, time::Duration};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use ccd_lcamv06::{BaudRate, Frame, CCD};
use cli::*;

fn main() -> Result<()> {
    simple_eyre::install()?;
    let cli = Cli::parse();
    env_logger::init();

    match &cli.command {
        Commands::List => list_serial(),
        Commands::CCDVersion(conf) => get_version(conf),
        Commands::Read(subcomm) => match &subcomm.command {
            ReadCommands::Single(conf) => get_single_reading(conf),
            //ReadCommands::Duration(conf) => get_duration_reading(conf),
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

fn ccd_over_serial(serial_path: &str) -> Result<CCD<Box<dyn SerialPort>>> {
    let port = serialport::new(
        serial_path,
        ToPrimitive::to_u32(&BaudRate::default()).unwrap(),
    )
    .timeout(Duration::from_millis(100))
    .open()
    .map_err(|_| eyre!("Could not open serial port"))?;
    Ok(CCD::new(port))
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

fn frame_to_hex(frame: &Frame) -> String {
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

/*
fn frames_to_hex(frames: &[Frame]) -> String {
    frames
        .iter()
        .map(frame_to_hex)
        .collect::<Vec<_>>()
        // Separate each frame by 2 empty lines
        .join("\n\n\n")
}
*/

fn frame_to_csv(frame: &Frame) -> String {
    frame
        .iter()
        .map(|pixel| pixel.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/*
fn frames_to_csv(frames: &[Frame]) -> String {
    frames
        .iter()
        .map(frame_to_csv)
        .collect::<Vec<_>>()
        .join("\n")
}
*/

/*
/// Gets readings for specified duration of time
fn get_duration_reading(conf: &DurationReadingConf) -> Result<()> {
    let mut frames: Vec<_> = Vec::new();
    let timeout = sleep(Duration::from_secs(conf.duration.into()));
    tokio::pin!(timeout);

    let mut ccd = ccd_over_serial(&conf.reading.serial.serial)?;

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
*/

fn get_single_reading(conf: &SingleReadingConf) -> Result<()> {
    let mut ccd = ccd_over_serial(&conf.serial.serial)?;
    let frame = ccd.get_frame()?;

    let mut out = File::create(&conf.output)?;
    let data = match conf.format {
        OutputFormat::CSV => frame_to_csv(&frame),
        OutputFormat::Hex => frame_to_hex(&frame),
    };
    out.write_all(&data.as_bytes())?;
    Ok(())
}

fn get_version(conf: &SerialConf) -> Result<()> {
    let mut ccd = ccd_over_serial(&conf.serial)?;
    let version_details = ccd.get_version()?;
    println!("{}", version_details);
    Ok(())
}

fn get_baud_rate(conf: &SerialConf) -> Result<()> {
    let mut ccd = ccd_over_serial(&conf.serial)?;
    let baud_rate = ccd.get_baudrate()?.to_u32().unwrap();
    println!("Current baud rate: {}", baud_rate);
    Ok(())
}

fn set_baud_rate(conf: &SetBaudRateConf) -> Result<()> {
    let mut ccd = ccd_over_serial(&conf.serial.serial)?;
    ccd.set_baudrate(conf.baud_rate)?;
    Ok(())
}

fn get_avg_time(conf: &SerialConf) -> Result<()> {
    let mut ccd = ccd_over_serial(&conf.serial)?;
    println!("Current \"average time\": {}", ccd.get_avg_time()?);
    Ok(())
}

fn set_avg_time(conf: &SetAvgTimeConf) -> Result<()> {
    let mut ccd = ccd_over_serial(&conf.serial.serial)?;
    ccd.set_avg_time(conf.average_time)?;
    Ok(())
}

fn get_exp_time(conf: &SerialConf) -> Result<()> {
    let mut ccd = ccd_over_serial(&conf.serial)?;
    println!("Current \"exposure time\": {}", ccd.get_exp_time()?);
    Ok(())
}

fn set_exp_time(conf: &SetExpTimeConf) -> Result<()> {
    let mut ccd = ccd_over_serial(&conf.serial.serial)?;
    ccd.set_exp_time(conf.exposure_time)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccd_lcamv06::FRAME_PIXEL_COUNT;

    #[test]
    fn convert_frame_to_hex() {
        let frame: Frame = [u16::from_be_bytes([0xA1, 0xB2]); FRAME_PIXEL_COUNT];
        let hex = frame_to_hex(&frame);
        let hex_lines: Vec<_> = hex.split("\n").collect();
        assert_eq!(hex_lines[0], "A1B2 A1B2 A1B2 A1B2");
    }

    #[test]
    fn convert_frame_to_csv() {
        let frame: Frame = [1000; FRAME_PIXEL_COUNT];
        let csv = frame_to_csv(&frame);
        let csv_fields: Vec<_> = csv.split(",").collect();
        assert_eq!(csv_fields[0], "1000");
    }
}
