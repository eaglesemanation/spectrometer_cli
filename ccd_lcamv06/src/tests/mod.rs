use super::*;

#[test]
fn decode_single_reading() {
    let res = decode_from_string(include_str!("./single_reading_example.txt"));
    // In ideal scenario input data should be a flat line and all values equal,
    // but this is a real world data and there is some noise.
    // Instead test if standard deviation is not too big
    if let Some(Ok(Response::SingleReading(frame))) = res.first() {
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
fn decode_stream() {
    let res = decode_from_string(include_str!("stream_example.txt"));
    // At the very least first package should be a frame
    assert!(if let Some(Ok(Response::SingleReading(_))) = res.first() {
        true
    } else {
        false
    });
    assert_eq!(res.len(), 116);
}
