use crate::{
    hex_parser::{decode_from_string, parse_hex_str},
    response_parser::{align_response, parse_response},
    types::Response,
};

use nom::{Err::Incomplete, Needed};

#[test]
fn decode_single_package() {
    let res = decode_from_string(include_str!("single_package_example.txt")).unwrap();
    // In ideal scenario input data should be a flat line and all values equal,
    // but this is a real world data and there is some noise.
    // Instead test if standard deviation is not too big
    if let Some(Response::SingleReading(frame)) = res.first() {
        // Filter out flaky inputs
        let frame_slice = &frame[10..frame.len() - 10];
        let size = frame_slice.len() as f32;
        let mean = frame_slice
            .iter()
            .fold(0f32, |accum, x| accum + (*x as f32 / size));
        let deviation = (frame_slice
            .iter()
            .map(|val| {
                let diff = mean - *val as f32;
                diff * diff
            })
            .sum::<f32>()
            / size)
            .sqrt();
        assert!(deviation < 100 as f32);
    } else {
        panic!("Incorrect response type");
    }
}

#[test]
fn try_decoding_partial_package() {
    let (_, data) = parse_hex_str(include_str!("single_package_example.txt")).expect("Could not parse hex file");
    let (data, _) = align_response(&data).expect("Could not find a start of a valid package");
    let partial_data = &data[..data.len() - 10];
    assert_eq!(parse_response(partial_data), Err(Incomplete(Needed::new(10))));
}

#[test]
fn decode_multiple_packages() {
    let res = decode_from_string(include_str!("multiple_packages_example.txt")).unwrap();
    // At the very least first package should be a frame
    assert!(if let Some(Response::SingleReading(_)) = res.first() {
        true
    } else {
        false
    });
    assert_eq!(res.len(), 116);
}
