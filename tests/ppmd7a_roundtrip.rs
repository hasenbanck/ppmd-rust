use std::io::{Read, Write};

use ppmd_rust::{Ppmd7aDecoder, Ppmd7aEncoder};

fn sample() -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..20_000u32 {
        v.extend_from_slice(
            format!("line {} of the PPMd var.H round trip, {}\n", i, i % 7).as_bytes(),
        );
    }
    v.extend(0..=255u8);
    v
}

#[test]
fn ppmd7a_round_trips_with_an_end_marker() {
    let data = sample();
    let mut encoder = Ppmd7aEncoder::new(Vec::new(), 6, 16 << 20).unwrap();
    encoder.write_all(&data).unwrap();
    let packed = encoder.finish(true).unwrap();
    assert!(
        packed.len() < data.len() / 4,
        "{} of {}",
        packed.len(),
        data.len()
    );
    let mut decoder = Ppmd7aDecoder::new(&packed[..], 6, 16 << 20).unwrap();
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).unwrap();
    assert_eq!(out, data);
}

#[test]
fn ppmd7a_round_trips_to_a_known_length() {
    let data = sample();
    let mut encoder = Ppmd7aEncoder::new(Vec::new(), 8, 8 << 20).unwrap();
    encoder.write_all(&data).unwrap();
    let packed = encoder.finish(false).unwrap();
    let mut decoder = Ppmd7aDecoder::new(&packed[..], 8, 8 << 20).unwrap();
    let mut out = vec![0u8; data.len()];
    decoder.read_exact(&mut out).unwrap();
    assert_eq!(out, data);
}
