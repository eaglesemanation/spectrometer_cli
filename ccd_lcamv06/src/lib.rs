#![feature(array_chunks)]

use bytes::{Buf, BytesMut};
use futures::{SinkExt, StreamExt};
use lazy_static::lazy_static;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::ToPrimitive;
use regex::Regex;
use std::{
    fmt::{Debug, Display},
    io,
    str::{from_utf8, FromStr},
};
use strum::{EnumIter, IntoEnumIterator};
use thiserror::Error;
use tokio_serial::{SerialPort, SerialPortBuilderExt, SerialStream};
use tokio_util::codec::{Decoder, Encoder, Framed};

pub struct CCDCodec;

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

const HEAD_SIZE: usize = 5;
// FIXME: Get info on real padding values
const FRAME_SIZE: usize = 3694; // Amount of effective pixels
const PRE_PADDING: usize = 0;
const POST_PADDING: usize = 0;
const PIXEL_COUNT: usize = PRE_PADDING + FRAME_SIZE + POST_PADDING;
const CRC_SIZE: usize = 2;

pub enum Endianness {
    LittleEndian,
    BigEndian
}
const ENDIANNESS: Endianness = Endianness::LittleEndian;
pub const U16_FROM_BYTES: fn([u8; 2]) -> u16 = match ENDIANNESS {
    Endianness::LittleEndian => u16::from_le_bytes,
    Endianness::BigEndian => u16::from_be_bytes
};
pub const U16_TO_BYTES: fn(u16) -> [u8; 2] = match ENDIANNESS {
    Endianness::LittleEndian => u16::to_le_bytes,
    Endianness::BigEndian => u16::to_be_bytes
};

impl Encoder<Command> for CCDCodec {
    type Error = Error;

    fn encode(&mut self, cmd: Command, dst: &mut BytesMut) -> Result<(), Self::Error> {
        use Command::*;

        dst.reserve(HEAD_SIZE);
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

#[derive(PartialEq, Eq, Debug)]
pub struct VersionDetails {
    pub hardware_version: String,
    pub sensor_type: String,
    pub firmware_version: String,
    pub serial_number: String,
}

impl FromStr for VersionDetails {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(",").collect();
        if parts.len() != 4 {
            Err(Error::InvalidData(
                "Version info should consist of 4 parts".to_string(),
            ))
        } else {
            Ok(VersionDetails {
                hardware_version: parts[0].to_string(),
                sensor_type: parts[1].to_string(),
                firmware_version: parts[2].to_string(),
                serial_number: parts[3].to_string(),
            })
        }
    }
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

pub type Frame = [u16; FRAME_SIZE];

#[derive(PartialEq, Eq, Debug)]
pub enum Response {
    SingleReading(Frame),
    ExposureTime(u16),
    AverageTime(u8),
    SerialBaudRate(BaudRate),
    VersionInfo(VersionDetails),
}

impl Decoder for CCDCodec {
    type Item = Response;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        use Error::*;
        use Response::*;

        if src.len() < HEAD_SIZE {
            // Wait until at least head is available
            return Ok(None);
        }
        let mut head = [0u8; HEAD_SIZE];
        head.copy_from_slice(&src[..HEAD_SIZE]);
        // Return without a header, probably version info
        if head[0] != 0x81 {
            return self.decode_version_info(src);
        }

        // Header should be present, either 5 byte command, or full frame
        match head[1] {
            // SingleReading
            // Orders of magnitude larger than standard 5 byte command, advance buffer only after
            // full frame captured
            0x01 => {
                let scan_size: usize = U16_FROM_BYTES(head[2..4].try_into().unwrap()).into();
                if (scan_size == 0 || scan_size == PIXEL_COUNT * 2) && head[4] == 0x00 {
                    // Cannot advance source buffer in case frame is not fully allocated yet
                    self.decode_frame(src)
                } else {
                    Err(InvalidData(format!(
                        "Unexpected scan size of {}",
                        scan_size
                    )))
                }
            }
            // ExposureTime
            0x02 => {
                src.advance(HEAD_SIZE);
                // Safe to unwrap as long as HEAD_SIZE > 4
                let exp_t = U16_FROM_BYTES(head[2..3].try_into().unwrap());
                match head[4] {
                    0xff => Ok(Some(ExposureTime(exp_t))),
                    _ => Err(UnexpectedEop),
                }
            }
            // AverageTime
            0x0e => {
                src.advance(HEAD_SIZE);
                let avg_t = head[2];
                match (head[3], head[4]) {
                    (0x00, 0xff) => Ok(Some(AverageTime(avg_t))),
                    _ => Err(UnexpectedEop),
                }
            }
            // SerialBaudRate
            0x16 => {
                src.advance(HEAD_SIZE);
                BaudRate::try_from_code(head[2])
                    .map_err(|_| InvalidBaudRate)
                    .and_then(|baud_rate| Ok(Some(SerialBaudRate(baud_rate))))
            }
            _ => {
                src.advance(HEAD_SIZE);
                Err(InvalidData(
                    "Unexpected command code for return value".to_string(),
                ))
            }
        }
    }
}

impl CCDCodec {
    fn decode_version_info(&mut self, src: &mut BytesMut) -> Result<Option<Response>, self::Error> {
        use Error::*;

        lazy_static! {
            static ref VERSION_INFO_RE: Regex = Regex::new(r"^HdInfo:((?:.*,){3}\d{12})").unwrap();
        }
        if let Some(caps) = VERSION_INFO_RE.captures(from_utf8(src).unwrap_or("")) {
            let version_info = caps
                .get(1)
                .ok_or(InvalidData("Could not parse version info".to_string()))
                .and_then(|m| VersionDetails::from_str(m.as_str()))?;
            src.advance(caps.get(0).unwrap().end());
            return Ok(Some(Response::VersionInfo(version_info)));
        }
        if let Some(idx) = src.iter().position(|b| *b == 0x81) {
            // Align buffer with the first received message
            src.advance(idx);
            return Ok(None);
        }
        if src.len() < 64 {
            // Let buffer fill a bit before deciding that it's garbage
            return Ok(None);
        }
        return Err(InvalidData(
            "Could not find a structured response".to_string(),
        ));
    }

    fn decode_frame(&mut self, src: &mut BytesMut) -> Result<Option<Response>, self::Error> {
        use Error::*;

        let package_size = HEAD_SIZE + PIXEL_COUNT * 2 + CRC_SIZE;
        if src.len() < package_size {
            if src.capacity() < package_size {
                // Preallocate space for a frame
                src.reserve(package_size - src.len())
            }
            Ok(None)
        } else {
            let scan = &src[HEAD_SIZE..package_size - CRC_SIZE];
            let crc = scan
                .iter()
                .fold(0u16, |accum, val| accum.wrapping_add(*val as u16));
            // Safe to unwrap, verified by if statement above
            let expected_crc =
                U16_FROM_BYTES(src[package_size - 2..package_size].try_into().unwrap());
            if crc != expected_crc {
                return Err(InvalidData(format!(
                    "Invalid CRC, expected {}, got {}",
                    expected_crc, crc
                )));
            }
            let frame = scan[PRE_PADDING * 2..(PRE_PADDING + FRAME_SIZE) * 2]
                // Split into owned byte arrays of size 2
                .array_chunks::<2>()
                .cloned()
                // Convert iterator over [u8; 2] into u16 assuming little-endian
                .map(U16_FROM_BYTES)
                // Convert iterator over u16 into [u16; _]
                .collect::<Vec<u16>>()
                .try_into()
                .unwrap();

            src.advance(package_size);
            Ok(Some(Response::SingleReading(frame)))
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_encoding() {
        let mut codec = CCDCodec {};
        let mut dst = BytesMut::with_capacity(0);

        codec
            .encode(Command::SetIntegrationTime(10), &mut dst)
            .unwrap();

        assert_eq!(&dst[..], &[0x81, 0x03, 0x00, 0x0a, 0xff]);
    }

    #[test]
    fn expected_decoding() {
        let mut codec = CCDCodec {};
        let mut src = BytesMut::with_capacity(0);
        // Encoded Response::VersionInfo
        src.extend_from_slice("HdInfo:LCAM_V8.4.2,S11639,V4.2,202111161548".as_bytes());

        let res = codec.decode(&mut src).unwrap().unwrap();
        assert_eq!(
            res,
            Response::VersionInfo(VersionDetails {
                hardware_version: "LCAM_V8.4.2".to_string(),
                sensor_type: "S11639".to_string(),
                firmware_version: "V4.2".to_string(),
                serial_number: "202111161548".to_string(),
            })
        );

        // Encoded Response::SingleReading
        let [len_upper, len_lower] = U16_TO_BYTES((PIXEL_COUNT * 2) as u16);
        // Head
        src.extend_from_slice(&[0x81, 0x01, len_upper, len_lower, 0x00]);
        // Full data block
        let data: [u8; PIXEL_COUNT * 2] = core::array::from_fn(|i| {
            if i < PRE_PADDING * 2 {
                // Prefix dummy pixels
                0u8
            } else if i < (PRE_PADDING + FRAME_SIZE) * 2 {
                // Actual data, check for CRC overflow behaviour
                if i % 2 == 0 {
                    0xAB
                } else {
                    0xCD
                }
            } else {
                // Postfix dummy pixels
                0u8
            }
        });
        // Calculate correct CRC
        let crc = data
            .iter()
            .fold(0u16, |acc, el| acc.wrapping_add((*el).into()));
        src.extend_from_slice(&data);
        src.extend_from_slice(&U16_TO_BYTES(crc));

        let res = codec.decode(&mut src).unwrap().unwrap();
        assert_eq!(
            res,
            Response::SingleReading([U16_FROM_BYTES([0xAB, 0xCD]); FRAME_SIZE])
        )
    }
}
