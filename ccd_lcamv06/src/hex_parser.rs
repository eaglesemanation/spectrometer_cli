use crate::{response::{Response, parser::{parse_response, align_response}}, error::Error};
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

pub fn parse_hex_str(input: &str) -> IResult<&str, Vec<u8>> {
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
    use claims::*;

    #[test]
    fn hex_byte_parser() {
        assert_ok_eq!(hex_byte("FF"), ("", 255));
        assert_ok_eq!(hex_byte("ff"), ("", 255));
        assert_err!(hex_byte("NH"));
    }

    #[test]
    fn hex_str_parser() {
        assert_ok_eq!(parse_hex_str("DEADBEEF"), ("", vec![0xDE, 0xAD, 0xBE, 0xEF]));
        assert_ok_eq!(parse_hex_str(" DE   AD BEEF    "), ("", vec![0xDE, 0xAD, 0xBE, 0xEF]));
        assert_err!(parse_hex_str("NOT HEX"));
        assert_err!(parse_hex_str("DE AD BE EF NO TH EX"));
    }
}
