use ccd_lcamv06::{BaudRate, error::Error};
use clap::{Args, Parser, Subcommand};
use num_traits::FromPrimitive;
use crate::{output::Output, serial::SerialConf};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Lists connected serial devices
    List,
    /// Get version info from CCD
    CCDVersion(SerialConf),
    /// Get readings from spectrometer
    Read(ReadCommand),
    /// Configure baud rate for UART, which is separate from USB port
    BaudRate(BaudRateCommand),
    /// "Average time" related commands, not sure what that really means
    AverageTime(AvgTimeCommand),
    /// "Exposure time" related commands, not sure how that's different from "average time"
    ExposureTime(ExpTimeCommand),
}

#[derive(Args)]
pub struct ReadCommand {
    #[clap(subcommand)]
    pub command: ReadCommands,
}

#[derive(Subcommand)]
pub enum ReadCommands {
    /// Get a single frame
    Single(SingleReadingConf),
    /// Get multiple frames
    Multi(MultiReadingConf)
}

#[derive(Args)]
pub struct SingleReadingConf {
    #[clap(flatten)]
    pub output: Output,

    #[clap(flatten)]
    pub serial: SerialConf,
}

#[derive(Args)]
pub struct MultiReadingConf {
    /// Amount of frames captured
    #[clap(value_parser, default_value = "50")]
    pub count: usize,

    #[clap(flatten)]
    pub output: Output,

    #[clap(flatten)]
    pub serial: SerialConf,
}

#[derive(Args)]
pub struct BaudRateCommand {
    #[clap(subcommand)]
    pub command: BaudRateCommands,
}

#[derive(Subcommand)]
pub enum BaudRateCommands {
    /// Get current baud rate
    Get(SerialConf),
    /// Set baud rate
    Set(SetBaudRateConf),
}

#[derive(Args)]
pub struct SetBaudRateConf {
    /// New baud rate on UART
    #[clap(value_parser = parse_baud_rate)]
    pub baud_rate: BaudRate,
    #[clap(flatten)]
    pub serial: SerialConf,
}

fn parse_baud_rate(s: &str) -> Result<BaudRate, Error> {
    s.parse()
        .or(Err(()))
        .and_then(|n| FromPrimitive::from_u32(n).ok_or(()))
        .map_err(|_| Error::InvalidBaudRate)
}

#[derive(Args)]
pub struct AvgTimeCommand {
    #[clap(subcommand)]
    pub command: AvgTimeCommands,
}

#[derive(Subcommand)]
pub enum AvgTimeCommands {
    /// Get current "average time"
    Get(SerialConf),
    /// Set "average time"
    Set(SetAvgTimeConf),
}

#[derive(Args)]
pub struct SetAvgTimeConf {
    /// New "average time"
    #[clap(value_parser)]
    pub average_time: u8,
    #[clap(flatten)]
    pub serial: SerialConf,
}

#[derive(Args)]
pub struct ExpTimeCommand {
    #[clap(subcommand)]
    pub command: ExpTimeCommands,
}

#[derive(Subcommand)]
pub enum ExpTimeCommands {
    /// Get current "exposure time"
    Get(SerialConf),
    /// Set "exposure time"
    Set(SetExpTimeConf),
}

#[derive(Args)]
pub struct SetExpTimeConf {
    /// New "exposure time"
    #[clap(value_parser)]
    pub exposure_time: u16,
    #[clap(flatten)]
    pub serial: SerialConf,
}
