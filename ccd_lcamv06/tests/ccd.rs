use utilities::{
    SINGLE_PACKAGE, MockIO
};
use ccd_lcamv06::CCD;
use std::io::Write;

#[test]
fn decode_single_package() {
    let mut mock_io = MockIO::new();
    mock_io.expect_write().returning(|msg| Ok(msg.len()));
    mock_io.expect_read().returning(move |mut buf| {
        buf.write(&SINGLE_PACKAGE)
    });
    let mut ccd = CCD::new(mock_io);

    // In ideal scenario input data should be a flat line and all values equal,
    // but this is a real world data and there is some noise.
    // Instead test if standard deviation is not too big
    let frame = ccd.get_frame().unwrap();
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
}
