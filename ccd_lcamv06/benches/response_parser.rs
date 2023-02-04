use utilities::{MockIO, SINGLE_PACKAGE};
use std::io::Write;
use ccd_lcamv06::{IoAdapter, StdIoAdapter};
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_decoding_packages(c: &mut Criterion) {
    let mut mock_io = MockIO::new();
    mock_io.expect_write().returning(|msg| Ok(msg.len()));
    mock_io.expect_read().returning(move |mut buf| {
        buf.write(&SINGLE_PACKAGE)
    });
    let mut ccd = StdIoAdapter::new(mock_io).open_ccd();

    c.bench_function("single package", |b| b.iter(|| ccd.get_frame()));
}

criterion_group!(benches, bench_decoding_packages);
criterion_main!(benches);
