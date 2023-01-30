use lazy_static::lazy_static;
use manifest_dir_macros::exist_relative_path;
use mockall::mock;
use nom::{
    bytes::complete::take_while_m_n,
    character::complete::space0,
    combinator::{all_consuming, map_res},
    multi::many1,
    sequence::delimited,
    IResult,
};
use std::io::{Read, Write};

/// Decodes a pair of chars formatted as hex into a byte. For example "FF" -> 255
fn hex_byte(input: &str) -> IResult<&str, u8> {
    map_res(
        take_while_m_n(2, 2, |c: char| c.is_ascii_hexdigit()),
        |hex| u8::from_str_radix(hex, 16),
    )(input)
}

fn parse_hex_str(input: &str) -> IResult<&str, Vec<u8>> {
    all_consuming(many1(delimited(space0, hex_byte, space0)))(input)
}

lazy_static! {
    pub static ref SINGLE_PACKAGE: Vec<u8> = {
        let hex_str = include_str!(exist_relative_path!(
            "resources/test/single_package_example.txt"
        ));
        let (_, data) = parse_hex_str(hex_str)
            .expect("Failed to parse resources/test/single_package_example.txt");
        data
    };
    pub static ref MULTIPLE_PACKAGES: Vec<u8> = {
        let hex_str = include_str!(exist_relative_path!(
            "resources/test/multiple_packages_example.txt"
        ));
        let (_, data) = parse_hex_str(hex_str)
            .expect("Failed to parse resources/test/multiple_packages_example.txt");
        data
    };
}

mock! {
    pub IO {}
    impl Read for IO {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;
    }
    impl Write for IO {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;
        fn flush(&mut self) -> std::io::Result<()>;
    }
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
        assert_ok_eq!(
            parse_hex_str("DEADBEEF"),
            ("", vec![0xDE, 0xAD, 0xBE, 0xEF])
        );
        assert_ok_eq!(
            parse_hex_str(" DE   AD BEEF    "),
            ("", vec![0xDE, 0xAD, 0xBE, 0xEF])
        );
        assert_err!(parse_hex_str("NOT HEX"));
        assert_err!(parse_hex_str("DE AD BE EF NO TH EX"));
    }
}
