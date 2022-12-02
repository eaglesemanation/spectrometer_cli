use core::fmt::Display;
use core::ops::RangeFrom;
use core::str::from_utf8;

use nom::{
    branch::alt,
    bytes::streaming::{tag, take, take_till1, take_while1},
    combinator::{map, peek},
    multi::fill,
    number::streaming::{be_u16, be_u8},
    sequence::{terminated, tuple},
    IResult, InputIter, InputLength, Slice,
};
use num_derive::{FromPrimitive, ToPrimitive};

#[derive(ToPrimitive, FromPrimitive, Debug, PartialEq, Eq, Clone, Copy)]
pub enum BaudRate {
    Baud115200 = 115200,
    Baud384000 = 384000,
    Baud921600 = 921600,
}

impl BaudRate {
    // u8 is ambiguous type, avoid implementing TryFrom trait to make conversion from code more
    // explicit
    fn try_from_code(value: u8) -> Result<Self, &'static str> {
        use BaudRate::*;
        match value {
            0x01 => Ok(Baud115200),
            0x02 => Ok(Baud384000),
            0x03 => Ok(Baud921600),
            _ => Err("Invalid baud rate code"),
        }
    }

    // u8 is ambiguous type, avoid implementing From trait to make conversion into code more explicit
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
    map(tag("HdInfo:"), |_| ())(input)
}

fn version_response<'a>(input: &'a [u8]) -> IResult<&'a [u8], Response<'a>> {
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
        Response::VersionInfo(VersionDetails {
            hardware_version,
            sensor_type,
            firmware_version,
            serial_number,
        }),
    ))
}

// Each reading is prefixed and postfixed with garbage data, which will be dropped
const FRAME_PIXEL_PREFIX: usize = 0;
const FRAME_PIXEL_POSTFIX: usize = 0;
// Amount of actual pixels in a single frame
pub const FRAME_PIXEL_COUNT: usize = 3694;
// Size of data part of SingleReading package
const FRAME_TOTAL_COUNT: usize = FRAME_PIXEL_PREFIX + FRAME_PIXEL_COUNT + FRAME_PIXEL_POSTFIX;

pub type Frame = [u16; FRAME_PIXEL_COUNT];

#[derive(PartialEq, Eq, Debug)]
pub enum Response<'a> {
    SingleReading(Frame),
    ExposureTime(u16),
    AverageTime(u8),
    SerialBaudRate(BaudRate),
    VersionInfo(VersionDetails<'a>),
}

/// byte version of nom::character::streaming::satisfy
fn u8_satisfy<F, I, E: nom::error::ParseError<I>>(cond: F) -> impl Fn(I) -> IResult<I, u8, E>
where
    I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
    F: Fn(u8) -> bool,
{
    move |i: I| match (i).iter_elements().next().map(|b| (b, cond(b))) {
        None => Err(nom::Err::Incomplete(nom::Needed::new(1))),
        Some((_, false)) => Err(nom::Err::Error(E::from_error_kind(
            i,
            nom::error::ErrorKind::Digit,
        ))),
        Some((b, true)) => Ok((i.slice(1..), b)),
    }
}

fn binary_response_prefix(input: &[u8]) -> IResult<&[u8], ()> {
    map(u8_satisfy(|b| b == 0x81), |_| ())(input)
}

/// Takes a byte slice and drops bytes until first valid prefix of a response
pub fn align_response(input: &[u8]) -> IResult<&[u8], ()> {
    for i in 0..input.len() {
        match peek(alt((binary_response_prefix, version_response_prefix)))(&input[i..]) {
            Ok(_) => return Ok((&input[i..], ())),
            _ => {}
        }
    }
    Err(nom::Err::Incomplete(nom::Needed::Unknown))
}

fn binary_response(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, _) = binary_response_prefix(input)?;
    let (input, cmd) = be_u8(input)?;
    match cmd {
        0x01 => single_frame_parser(input),
        0x02 => exposure_time_parser(input),
        0x0E => average_time_parser(input),
        0x16 => serial_baud_rate_parser(input),
        _ => Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Digit,
        ))),
    }
}

fn single_frame_parser(input: &[u8]) -> IResult<&[u8], Response> {
    // Parse head
    let (input, scan_size) = be_u16(input)?;
    if scan_size != (FRAME_TOTAL_COUNT as u16 * 2) {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Digit,
        )));
    }
    let (input, _) = u8_satisfy(|b| b == 0x00)(input)?;

    // Calculate CRC on individual bytes, each pixel is 2 bytes long
    let crc = input[..FRAME_TOTAL_COUNT * 2]
        .iter()
        .fold(0u16, |accum, val| accum.wrapping_add(*val as u16));

    // Parse data
    let mut data = [0u16; FRAME_TOTAL_COUNT];
    let (input, ()) = fill(be_u16, &mut data)(input)?;
    let (input, expected_crc) = be_u16(input)?;
    if crc != expected_crc {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Digit,
        )));
    }
    Ok((input, Response::SingleReading(data)))
}

fn exposure_time_parser(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, exposure_time) = be_u16(input)?;
    let (input, _) = u8_satisfy(|b| b == 0xFF)(input)?;
    Ok((input, Response::ExposureTime(exposure_time)))
}

fn average_time_parser(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, average_time) = be_u8(input)?;
    let (input, _) = u8_satisfy(|b| b == 0x00)(input)?;
    let (input, _) = u8_satisfy(|b| b == 0xFF)(input)?;
    Ok((input, Response::AverageTime(average_time)))
}

fn serial_baud_rate_parser(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, baud_rate_code) = be_u8(input)?;
    let (input, _) = u8_satisfy(|b| b == 0x00)(input)?;
    let (input, _) = u8_satisfy(|b| b == 0xFF)(input)?;

    if let Ok(baud_rate) = BaudRate::try_from_code(baud_rate_code) {
        Ok((input, Response::SerialBaudRate(baud_rate)))
    } else {
        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Digit,
        )))
    }
}

/// Takes aligned input and parses it as either as a byte stream, or as plain text in case of
/// version info response
pub fn parse_response(input: &[u8]) -> IResult<&[u8], Response> {
    alt((binary_response, version_response))(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::{
        error::{Error, ErrorKind},
        Needed,
    };

    fn hex_to_bytes(hex: &str) -> Vec<u8> {
        hex.split(&[' ', '\n'][..])
            .filter_map(|b| {
                if b.len() == 2 && b.chars().all(|c| c.is_ascii_hexdigit()) {
                    // Verified valid byte, safe to unwrap
                    Some(u8::from_str_radix(b, 16).unwrap())
                } else {
                    None
                }
            })
            .collect()
    }

    #[test]
    fn decode_binary_prefix() {
        // Expected prefix
        assert_eq!(binary_response_prefix(&[0x81u8]), Ok(((&[] as &[u8]), ())));
        // Invalid prefix
        assert_eq!(
            binary_response_prefix(&[0x80u8]),
            Err(nom::Err::Error(Error {
                input: [0x80u8].as_slice(),
                code: ErrorKind::Digit
            }))
        );
        // Prefix did not arrive yet
        assert_eq!(
            binary_response_prefix(&[] as &[u8]),
            Err(nom::Err::Incomplete(Needed::new(1)))
        );
    }

    #[test]
    fn decode_single_reading() {
        let input = hex_to_bytes(include_str!("./single_reading_example.txt"));
        let parse_res = binary_response(input.as_slice());
        //assert!(parse_res.is_ok());
        let (input, resp) = parse_res.unwrap();
        assert!(input.len() == 0);
        if let Response::SingleReading(frame) = resp {
            assert!(frame[0] > 0);
        } else {
            panic!("Expected SingleReading response");
        }
    }

    #[test]
    fn decode_baud_rate() {
        assert_eq!(
            binary_response(&[0x81u8, 0x16, 0x01, 0x00, 0xFF]),
            Ok((
                (&[] as &[u8]),
                Response::SerialBaudRate(BaudRate::Baud115200)
            ))
        );
        // Invalid baud rate code
        assert!(binary_response(&[0x81u8, 0x16, 0xFF, 0x00, 0xFF]).is_err());
    }

    #[test]
    fn decode_exposure_time() {
        assert_eq!(
            binary_response(&[0x81u8, 0x02, 0xAB, 0xCD, 0xFF]),
            Ok(((&[] as &[u8]), Response::ExposureTime(0xABCD)))
        );
        // Invalid suffix
        assert!(binary_response(&[0x81, 0x02, 0xAB, 0xCD, 0x00]).is_err());
    }

    #[test]
    fn decode_average_time() {
        assert_eq!(
            binary_response(&[0x81u8, 0x0E, 0xAB, 0x00, 0xFF]),
            Ok(((&[] as &[u8]), Response::AverageTime(0xAB)))
        );
        // Incorrect low byte
        assert!(binary_response(&[0x81u8, 0x0E, 0xAB, 0xCD, 0xFF]).is_err());
    }

    #[test]
    fn test_align_response() {
        assert_eq!(
            align_response("   HdInfo:".as_bytes()),
            Ok(("HdInfo:".as_bytes(), ()))
        );
        assert_eq!(
            align_response(&([0xDE, 0xAD, 0xBE, 0xEF, 0x81] as [u8; 5])),
            Ok((&[0x81u8] as &[u8], ()))
        );
        // Allow any kind of garbage until known valid response arrives
        assert_eq!(
            align_response("   HDInfo:".as_bytes()),
            Err(nom::Err::Incomplete(Needed::Unknown))
        );
    }

    #[test]
    fn decode_version_info() {
        assert_eq!(
            // Encoded Response::VersionInfo
            version_response("HdInfo:LCAM_V8.4.2,S11639,V4.2,202111161548".as_bytes()),
            Ok((
                "".as_bytes(),
                Response::VersionInfo(VersionDetails {
                    hardware_version: "LCAM_V8.4.2",
                    sensor_type: "S11639",
                    firmware_version: "V4.2",
                    serial_number: "202111161548",
                })
            ))
        );
    }
}
