use crate::{types::Response, error::Error, response_parser::{parse_response, align_response}};
use nom::{
    bytes::complete::take_while_m_n, character::complete::space0, combinator::{map_res, all_consuming},
    multi::many1, sequence::delimited, IResult,
};

/// Decodes a pair of chars formatted as hex into a byte. For example "FF" -> 255
fn hex_byte(input: &str) -> IResult<&str, u8> {
    map_res(
        take_while_m_n(2, 2, |c: char| c.is_ascii_hexdigit()),
        |hex| u8::from_str_radix(hex, 16),
    )(input)
}

pub(crate) fn parse_hex_str(input: &str) -> IResult<&str, Vec<u8>> {
    all_consuming(many1(delimited(space0, hex_byte, space0)))(input)
}

pub fn decode_from_string(input: &str) -> Result<Vec<Response>, Error> {
    let (_, data) =
        parse_hex_str(input).map_err(|_| Error::InvalidData("Could not parse hex file"))?;
    let (mut data, _) = align_response(&data)
        .map_err(|_| Error::InvalidData("Could not find a start of a valid package"))?;
    let mut parsed_responses = Vec::new();
    loop {
        match parse_response(data) {
            Ok((new_data, resp)) => {
                data = new_data;
                parsed_responses.push(resp.to_owned());
            }
            Err(nom::Err::Incomplete(_)) => break,
            Err(_) => return Err(Error::InvalidData("Could not parse package correctly")),
        }
    }
    Ok(parsed_responses)
}


#[cfg(test)]
mod tests {
    use super::*;
    use nom::{error::{make_error, ErrorKind}, Err::Error};
    use pretty_assertions::assert_eq;

    #[test]
    fn hex_byte_parser() {
        assert_eq!(hex_byte("FF"), Ok(("", 255)));
        assert_eq!(hex_byte("ff"), Ok(("", 255)));
        assert!(hex_byte("NH").is_err());
    }

    #[test]
    fn hex_str_parser() {
        assert_eq!(parse_hex_str("DEADBEEF"), Ok(("", vec![0xDE, 0xAD, 0xBE, 0xEF])));
        assert_eq!(parse_hex_str(" DE   AD BEEF    "), Ok(("", vec![0xDE, 0xAD, 0xBE, 0xEF])));
        assert_eq!(parse_hex_str("NOT HEX"), Err(Error(make_error("NOT HEX", ErrorKind::TakeWhileMN))));
        assert_eq!(parse_hex_str("DE AD BE EF NO TH EX"), Err(Error(make_error("NO TH EX", ErrorKind::Eof))));
    }
}
