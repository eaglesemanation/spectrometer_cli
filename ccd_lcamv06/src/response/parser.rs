use core::{num::NonZeroUsize, ops::RangeFrom};

use nom::{
    branch::alt,
    combinator::{map, peek},
    multi::fill,
    number::streaming::{be_u16, be_u8},
    IResult, InputIter, InputLength, Slice,
};

use crate::config::BaudRate;
use super::version_parser::*;
use super::{Response, FRAME_TOTAL_COUNT};

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

fn package_prefix(input: &[u8]) -> IResult<&[u8], ()> {
    map(u8_satisfy(|b| b == 0x81), |_| ())(input)
}

fn package_parser(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, _) = package_prefix(input)?;
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
    // Check if buffer has all data required + a byte for CRC
    const REMAINING_LEN: usize = (FRAME_TOTAL_COUNT + 1) * 2;
    if input.len() < REMAINING_LEN {
        // Can safely unwrap due to check
        let needed = NonZeroUsize::new(REMAINING_LEN - input.len()).unwrap();
        return Err(nom::Err::Incomplete(nom::Needed::Size(needed)));
    }

    // Calculate CRC on individual bytes, each pixel is 2 bytes long
    let _crc = input[..FRAME_TOTAL_COUNT * 2]
        .iter()
        .fold(0u16, |accum, val| accum.wrapping_add(*val as u16));

    // Parse data
    let mut data = [0u16; FRAME_TOTAL_COUNT];
    let (input, ()) = fill(be_u16, &mut data)(input)?;
    let (input, _expected_crc) = be_u16(input)?;
    // TODO: Figure out why some packages include wrong CRC
    /*
    if crc != expected_crc {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Digit,
        )));
    }
    */
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

/// Takes a byte slice and drops bytes until first valid prefix of a response
pub fn align_response(input: &[u8]) -> IResult<&[u8], ()> {
    for i in 0..input.len() {
        match peek(alt((package_prefix, version_details_prefix)))(&input[i..]) {
            Ok(_) => return Ok((&input[i..], ())),
            _ => {}
        }
    }
    Err(nom::Err::Incomplete(nom::Needed::Unknown))
}

/// Takes aligned input and parses it as either as a byte stream, or as plain text in case of
/// version info response
pub fn parse_response(input: &[u8]) -> IResult<&[u8], Response> {
    alt((
        package_parser,
        map(version_details_parser, |vd| Response::VersionInfo(vd)),
    ))(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use BaudRate::*;
    use claims::*;
    use nom::{Err::Incomplete, Needed};

    #[test]
    fn decode_package_prefix() {
        // Expected prefix
        assert_eq!(package_prefix(&[0x81u8]), Ok(((&[] as &[u8]), ())));
        // Invalid prefix
        assert_err!(package_prefix(&[0x80u8]));
        // Prefix did not arrive yet
        assert_err_eq!(package_prefix(&[] as &[u8]), Incomplete(Needed::new(1)));
    }

    #[test]
    fn decode_baud_rate() {
        assert_ok_eq!(
            package_parser(&[0x81u8, 0x16, 0x01, 0x00, 0xFF]),
            (&[] as &[u8], Response::SerialBaudRate(Baud115200))
        );
        // Invalid baud rate code
        assert_err!(package_parser(&[0x81u8, 0x16, 0xFF, 0x00, 0xFF]));
    }

    #[test]
    fn decode_exposure_time() {
        assert_ok_eq!(
            package_parser(&[0x81u8, 0x02, 0xAB, 0xCD, 0xFF]),
            (&[] as &[u8], Response::ExposureTime(0xABCD))
        );
        // Invalid suffix
        assert_err!(package_parser(&[0x81, 0x02, 0xAB, 0xCD, 0x00]));
    }

    #[test]
    fn decode_average_time() {
        assert_ok_eq!(
            package_parser(&[0x81u8, 0x0E, 0xAB, 0x00, 0xFF]),
            (&[] as &[u8], Response::AverageTime(0xAB))
        );
        // Incorrect low byte
        assert_err!(package_parser(&[0x81u8, 0x0E, 0xAB, 0xCD, 0xFF]));
    }

    #[test]
    fn test_align_response() {
        assert_ok_eq!(
            align_response("   HdInfo:".as_bytes()),
            ("HdInfo:".as_bytes(), ())
        );
        // Don't do anything if package is already aligned
        assert_ok_eq!(
            align_response("HdInfo:".as_bytes()),
            ("HdInfo:".as_bytes(), ())
        );
        assert_ok_eq!(
            align_response(&([0xDE, 0xAD, 0xBE, 0xEF, 0x81] as [u8; 5])),
            (&[0x81u8] as &[u8], ())
        );
        // Allow any kind of garbage until known valid response arrives
        assert_err_eq!(
            align_response("   HDInfo:".as_bytes()),
            Incomplete(Needed::Unknown)
        );
    }
}
