mod response_parser;
#[cfg(test)]
mod tests;

pub use response_parser::{align_response, parse_response};

use bytes::{Buf, BytesMut};
use futures::{SinkExt, StreamExt};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::ToPrimitive;
use std::{
    fmt::{Debug, Display},
    io,
};
use strum::{EnumIter, IntoEnumIterator};
use thiserror::Error;
use tokio_serial::{SerialPort, SerialPortBuilderExt, SerialStream};
use tokio_util::codec::{Decoder, Encoder, Framed};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Baud rate is not in range of accepted values: {}", BaudRate::iter().map(|b| b.to_string()).collect::<Vec<String>>().join(", "))]
    InvalidBaudRate,
    #[error("Could not automatically detect baud rate for selected serial device")]
    BaudAutoDetectFailed,
    #[error("Could not open a serial device with specified path")]
    InvalidSerialPath,
    #[error("Could not parse recieved data correctly: {0}")]
    InvalidData(String),
    #[error("Unexpected end of package")]
    UnexpectedEop,
    #[error("IO error: {0}")]
    IOError(#[from] io::Error),
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum TriggerMode {
    SoftTrigger = 0x00,
    ContiniousHardTrigger = 0x01,
    SingleHardTrigger = 0x02,
}

#[derive(ToPrimitive, FromPrimitive, EnumIter, Debug, PartialEq, Eq, Clone, Copy)]
pub enum BaudRate {
    Baud115200 = 115200,
    Baud384000 = 384000,
    Baud921600 = 921600,
}

impl Default for BaudRate {
    fn default() -> Self {
        BaudRate::Baud115200
    }
}

impl Display for BaudRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", *self as u32))
    }
}

impl BaudRate {
    fn try_from_code(c: u8) -> Result<Self, Error> {
        use BaudRate::*;
        match c {
            0x01 => Ok(Baud115200),
            0x02 => Ok(Baud384000),
            0x03 => Ok(Baud921600),
            _ => Err(Error::InvalidBaudRate),
        }
    }

    fn to_code(&self) -> u8 {
        use BaudRate::*;
        match *self {
            Baud115200 => 0x01,
            Baud384000 => 0x02,
            Baud921600 => 0x03,
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum Command {
    SingleRead,
    ContinuousRead,
    PauseRead,
    SetIntegrationTime(u16),
    SetTrigerMode(TriggerMode),
    GetExposureTime,
    GetVersion,
    SetAverageTime(u8),
    GetAverageTime,
    SetSerialBaudRate(BaudRate),
    GetSerialBaudRate,
}

impl Command {
    /// Convert command enum into byte code for encoding
    fn code(&self) -> u8 {
        use Command::*;

        match *self {
            SingleRead => 0x01,
            ContinuousRead => 0x02,
            SetIntegrationTime(_) => 0x03,
            PauseRead => 0x06,
            SetTrigerMode(_) => 0x07,
            GetVersion => 0x09,
            GetExposureTime => 0x0a,
            SetAverageTime(_) => 0x0c,
            GetAverageTime => 0x0e,
            SetSerialBaudRate(_) => 0x13,
            GetSerialBaudRate => 0x16,
        }
    }
}

/// Amount of real pixels in a single frame
pub const FRAME_PIXEL_COUNT: usize = 3694;
/// Each reading is prefixed and postfixed with "ghost" pixels, which can be dropped
const FRAME_PIXEL_PREFIX: usize = 0;
const FRAME_PIXEL_POSTFIX: usize = 0;
/// Amount of pixels in a single frame
const FRAME_TOTAL_COUNT: usize = FRAME_PIXEL_PREFIX + FRAME_PIXEL_COUNT + FRAME_PIXEL_POSTFIX;

/// CCD captured data
pub type Frame = [u16; FRAME_PIXEL_COUNT];

#[derive(PartialEq, Eq, Debug)]
pub enum Response {
    SingleReading(Frame),
    ExposureTime(u16),
    AverageTime(u8),
    SerialBaudRate(BaudRate),
    VersionInfo(VersionDetails),
}

pub struct CCDCodec;

impl Encoder<Command> for CCDCodec {
    type Error = Error;

    fn encode(&mut self, cmd: Command, dst: &mut BytesMut) -> Result<(), Self::Error> {
        use Command::*;

        dst.reserve(5);
        // Head + Command code
        dst.extend_from_slice(&[0x81, cmd.code()]);
        // Data
        match cmd {
            SetIntegrationTime(t) => dst.extend_from_slice(&t.to_be_bytes()),
            SetTrigerMode(m) => dst.extend_from_slice(&[m as u8, 0x00]),
            SetAverageTime(t) => dst.extend_from_slice(&[t as u8, 0x00]),
            SetSerialBaudRate(r) => dst.extend_from_slice(&[r.to_code(), 0x00]),
            _ => dst.extend_from_slice(&[0x00, 0x00]),
        }
        // Tail
        dst.extend_from_slice(&[0xff]);

        Ok(())
    }
}

impl Decoder for CCDCodec {
    type Item = Response;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        use Error::*;

        match parse_response(&src) {
            Err(nom::Err::Incomplete(_)) => Ok(None),
            Err(_) => Err(InvalidData("Could not parse response".to_string())),
            Ok((tail, resp)) => {
                src.advance(src.len() - tail.len());
                Ok(Some(resp))
            }
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct VersionDetails {
    pub hardware_version: String,
    pub sensor_type: String,
    pub firmware_version: String,
    pub serial_number: String,
}

impl Display for VersionDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            concat!(
                "Hardware version: {}\n",
                "Firmware version: {}\n",
                "Sensor type: {}\n",
                "Serial number: {}",
            ),
            self.hardware_version, self.firmware_version, self.sensor_type, self.serial_number
        ))
    }
}

/// Use a handler on a response from CCD codec only if resp matches resp_type
/// # Examples
/// ```
/// # use tokio;
/// use ccd_lcamv06::{Response, handle_ccd_response, VersionDetails};
/// # #[tokio::main]
/// # async fn main() {
/// # let ccd_response = Some(Ok(Response::VersionInfo(VersionDetails{
/// #   hardware_version: "1".to_string(),
/// #   firmware_version: "2".to_string(),
/// #   sensor_type: "3".to_string(),
/// #   serial_number: "4".to_string(),
/// # })));
/// handle_ccd_response!(
///     // Usually you would get this struct from Framed<StreamSerial, CCDCoded>
///     ccd_response,
///     Response::VersionInfo,
///     |info: VersionDetails| {
///         println!("{}", info);
///         Ok(())
///     }
/// ).unwrap();
/// # }
/// ```
#[macro_export]
macro_rules! handle_ccd_response {
    ($resp:expr, $resp_type:path, $handler:expr) => {
        match $resp {
            Some(Ok($resp_type(val))) => $handler(val),
            Some(Ok(_)) => Err($crate::Error::InvalidData(
                "Got unexpected type of response".to_string(),
            )),
            Some(Err(err)) => Err(err),
            None => Err($crate::Error::UnexpectedEop),
        }
    };
}

pub struct CCDConf {
    pub baud_rate: BaudRate,
    pub serial_path: String,
}

pub async fn try_new_ccd(conf: &CCDConf) -> Result<Framed<SerialStream, CCDCodec>, Error> {
    let mut current_baud: Option<BaudRate> = None;

    let port = tokio_serial::new(conf.serial_path.clone(), conf.baud_rate.to_u32().unwrap())
        .open_native_async()
        .map_err(|_| Error::InvalidSerialPath)?;
    let mut ccd = CCDCodec.framed(port);

    // Try detecting current baud rate using all supported baud rates
    for baud in BaudRate::iter() {
        ccd.get_mut()
            .set_baud_rate(baud.to_u32().unwrap())
            .map_err(|_| Error::BaudAutoDetectFailed)?;

        if let Err(_) = ccd.send(Command::GetSerialBaudRate).await {
            continue;
        }

        ccd.flush().await.map_err(|_| Error::BaudAutoDetectFailed)?;
        let resp = ccd.next().await;
        if let Some(Ok(Response::SerialBaudRate(b))) = resp {
            current_baud = Some(b);
            break;
        }
    }

    let current_baud = current_baud.ok_or(Error::BaudAutoDetectFailed)?;
    if current_baud != conf.baud_rate {
        ccd.send(Command::SetSerialBaudRate(conf.baud_rate))
            .await
            .map_err(|_| Error::BaudAutoDetectFailed)?;
    }

    ccd.get_mut()
        .set_baud_rate(conf.baud_rate.to_u32().unwrap())
        .map_err(|_| Error::BaudAutoDetectFailed)?;
    ccd.flush().await.map_err(|_| Error::BaudAutoDetectFailed)?;
    Ok(ccd)
}

pub fn decode_from_string(single_reading_hex: &str) -> Vec<Result<Response, Error>> {
    let mut codec = CCDCodec {};
    let mut src = BytesMut::with_capacity(0);

    let single_reading_iter = single_reading_hex.split(&[' ', '\n'][..]).filter_map(|b| {
        if b.len() == 2 && b.chars().all(|c| c.is_ascii_hexdigit()) {
            // Verified valid byte, safe to unwrap
            Some(u8::from_str_radix(b, 16).unwrap())
        } else {
            None
        }
    });
    src.extend(single_reading_iter);

    let mut resp = Vec::new();

    loop {
        match codec.decode(&mut src) {
            Ok(Some(pkg)) => resp.push(Ok(pkg)),
            Err(err) => resp.push(Err(err)),
            Ok(None) => break,
        }
    }
    resp
}
