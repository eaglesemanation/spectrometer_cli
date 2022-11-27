use core::fmt::Display;
use core::str::from_utf8;

use nom::{
    bytes::streaming::{tag, take, take_till1, take_while1},
    sequence::{terminated, tuple},
    IResult,
};
use num_derive::{FromPrimitive, ToPrimitive};

#[derive(ToPrimitive, FromPrimitive, Debug, PartialEq, Eq, Clone, Copy)]
pub enum BaudRate {
    Baud115200 = 115200,
    Baud384000 = 384000,
    Baud921600 = 921600,
}

#[derive(PartialEq, Eq, Debug)]
pub struct VersionDetails<'a> {
    pub hardware_version: &'a str,
    pub sensor_type: &'a str,
    pub firmware_version: &'a str,
    pub serial_number: &'a str,
}

impl<'a> Display for VersionDetails<'a> {
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

fn is_separator(c: u8) -> bool {
    c == b' ' || c == b','
}

fn word_with_separator<'a>(input: &[u8]) -> IResult<&[u8], &str> {
    let (input, b) = terminated(take_till1(is_separator), take_while1(is_separator))(input)?;
    // TODO: Handle error
    Ok((input, from_utf8(b).unwrap()))
}

fn version_response_prefix(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = tag("HdInfo:")(input)?;
    Ok((input, ()))
}

fn version_response<'a>(input: &'a [u8]) -> IResult<&'a [u8], VersionDetails<'a>> {
    let (input, (_, hardware_version, sensor_type, firmware_version, serial_number)) =
        tuple((
            // Prefix
            version_response_prefix,
            // Hardware info
            word_with_separator,
            // Sensor type
            word_with_separator,
            // Firmware version
            word_with_separator,
            // Serial number, should be a timestamp
            take("202111161548".len()),
        ))(input)?;
    // TODO: Handle error
    let serial_number = from_utf8(serial_number).unwrap();

    Ok((
        input,
        VersionDetails {
            hardware_version,
            sensor_type,
            firmware_version,
            serial_number,
        },
    ))
}

// Each reading is prefixed and postfixed with garbage data, which will be dropped
const PRE_PADDING: usize = 0;
const POST_PADDING: usize = 0;
pub const FRAME_SIZE: usize = 3694;

pub type Frame = [u16; FRAME_SIZE];

#[derive(PartialEq, Eq, Debug)]
pub enum Response<'a> {
    SingleReading(Frame),
    ExposureTime(u16),
    AverageTime(u8),
    SerialBaudRate(BaudRate),
    VersionInfo(VersionDetails<'a>),
}

fn binary_response_prefix(input: &[u8]) -> IResult<&[u8], ()> {
    
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_version_info() {
        assert_eq!(
            // Encoded Response::VersionInfo
            version_response("HdInfo:LCAM_V8.4.2,S11639,V4.2,202111161548".as_bytes()),
            Ok((
                "".as_bytes(),
                VersionDetails {
                    hardware_version: "LCAM_V8.4.2",
                    sensor_type: "S11639",
                    firmware_version: "V4.2",
                    serial_number: "202111161548",
                }
            ))
        );
    }
}
