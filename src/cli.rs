use crate::ccd_codec::{BaudError, BaudRate};
use clap::{Args, Parser, Subcommand};
use num_traits::FromPrimitive;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Args)]
pub struct SerialConf {
    /// Name of serial port that should be used
    #[clap(short, long, value_parser)]
    pub serial: String,
    /// Which baud rate to use
    #[clap(short, long, value_parser = parse_baud_rate)]
    pub baud_rate: Option<BaudRate>,
}

fn parse_baud_rate(s: &str) -> Result<BaudRate, BaudError> {
    s.parse()
        .or(Err(()))
        .and_then(|n| FromPrimitive::from_u32(n).ok_or(()))
        .map_err(|_| BaudError::IncorrectBaudRate)
}

#[derive(Subcommand)]
pub enum Commands {
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
pub struct ReadCommand {
    #[clap(subcommand)]
    pub command: ReadCommands,
}

#[derive(Subcommand)]
pub enum ReadCommands {
    /// Get a single frame
    Single(SingleReadingConf),
    /// Continiously get readings for specified duration
    Duration(DurationReadingConf),
}

#[derive(Args)]
pub struct SingleReadingConf {
    /// Path to a file where readings should be stored
    #[clap(short, long, value_parser, value_hint = clap::ValueHint::FilePath)]
    pub output: String,

    #[clap(flatten)]
    pub serial: SerialConf,
}

#[derive(Args)]
pub struct DurationReadingConf {
    /// Duration in seconds for which frames are continiously captured
    #[clap(short, long, value_parser, default_value = "3")]
    pub duration: u8,

    #[clap(flatten)]
    pub reading: SingleReadingConf,
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
