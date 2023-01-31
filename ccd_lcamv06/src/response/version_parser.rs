use core::str::from_utf8;

use nom::{
    bytes::streaming::{tag, take, take_till1, take_while1},
    combinator::map,
    sequence::{terminated, tuple},
    IResult,
};

use super::version_details::VersionDetails;

fn is_separator(c: u8) -> bool {
    c == b' ' || c == b','
}

fn word_with_separator(input: &[u8]) -> IResult<&[u8], &str> {
    let (input, b) = terminated(take_till1(is_separator), take_while1(is_separator))(input)?;
    // TODO: Handle error
    Ok((input, from_utf8(b).unwrap()))
}

pub(crate) fn version_details_prefix(input: &[u8]) -> IResult<&[u8], ()> {
    map(tag("HdInfo:"), |_| ())(input)
}

pub(crate) fn version_details_parser(input: &[u8]) -> IResult<&[u8], VersionDetails> {
    let (input, (_, hw_ver, sensor, fw_ver, serial)) = tuple((
        // Prefix
        version_details_prefix,
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
    let serial = from_utf8(serial).unwrap();

    // TODO: Handle error
    Ok((
        input,
        VersionDetails::try_new(hw_ver, sensor, fw_ver, serial).unwrap(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_version_details() {
        assert_eq!(
            // Encoded Response::VersionInfo
            version_details_parser("HdInfo:LCAM_V8.4.2,S11639,V4.2,202111161548".as_bytes()),
            Ok((
                "".as_bytes(),
                VersionDetails::try_new("LCAM_V8.4.2", "S11639", "V4.2", "202111161548").unwrap()
            ))
        );
    }
}
