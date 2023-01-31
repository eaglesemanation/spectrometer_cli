mod cli;

use clap::Parser;
use simple_eyre::Result;
use num_traits::ToPrimitive;
use std::{fs::File, io::Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use ccd_lcamv06::Frame;
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
                    format!("{b1:02X}{b2:02X}")
                })
                .collect::<Vec<_>>()
                // Separate each work with a space
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn frames_to_hex(frames: &[Frame]) -> String {
    frames
        .iter()
        .map(frame_to_hex)
        .collect::<Vec<_>>()
        // Separate each frame by 2 empty lines
        .join("\n\n\n")
}

fn frame_to_csv(frame: &Frame) -> String {
    frame
        .iter()
        .map(|pixel| pixel.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn frames_to_csv(frames: &[Frame]) -> String {
    frames
        .iter()
        .map(frame_to_csv)
        .collect::<Vec<_>>()
        .join("\n")
}

fn get_multiple_readings(conf: &MultiReadingConf) -> Result<()> {
    let mut ccd = conf.reading.serial.open_ccd()?;
    let mut frames: Vec<_> = Vec::new();
    frames.reserve(conf.count);

    ccd.get_frames(&mut frames)?;

    let mut out = File::create(&conf.reading.output)?;
    let data = match conf.reading.format {
        OutputFormat::Csv => {
            frames_to_csv(&frames)
        }
        OutputFormat::Hex => {
            frames_to_hex(&frames)
        }
    };
    out.write_all(data.as_bytes())?;

    Ok(())
}

fn get_single_reading(conf: &SingleReadingConf) -> Result<()> {
    let mut ccd = conf.serial.open_ccd()?;
    let frame = ccd.get_frame()?;

    let mut out = File::create(&conf.output)?;
    let data = match conf.format {
        OutputFormat::Csv => frame_to_csv(&frame),
        OutputFormat::Hex => frame_to_hex(&frame),
    };
    out.write_all(data.as_bytes())?;
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
