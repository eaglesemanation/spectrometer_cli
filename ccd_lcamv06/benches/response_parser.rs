use criterion::{criterion_group, criterion_main, Criterion};
use manifest_dir_macros::exist_relative_path;
use ccd_lcamv06::{response::parser::parse_response, hex_parser::parse_hex_str};

const SINGLE_PACKAGE: &'static str = include_str!(exist_relative_path!("resources/test/single_package_example.txt"));

fn bench_decoding_packages(c: &mut Criterion) {
    let (_, package) = parse_hex_str(SINGLE_PACKAGE).expect("Could not parse hex file");
    c.bench_function("single package", |b| b.iter(|| parse_response(&package)));
}

criterion_group!(benches, bench_decoding_packages);
criterion_main!(benches);

