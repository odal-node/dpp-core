//! W3C Bitstring Status List — decoding a published list and testing one bit.
//!
//! A verifier decodes the list it fetched and then asks about a single index, so
//! the two are timed separately: decode scales with the list, lookup does not.
//! Lists are almost entirely zero with a sparse run of set bits, which is what a
//! real list looks like and what gzip compresses best; a denser list decodes
//! more slowly than these figures.

use std::hint::black_box;
use std::io::Write;

use base64::Engine;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use dpp_vc::status_list::StatusList;
use flate2::{Compression, write::GzEncoder};

/// A bitstring of `bits` entries with every thousandth bit set.
fn bitstring(bits: usize) -> Vec<u8> {
    let mut raw = vec![0_u8; bits / 8];
    for index in (0..bits).step_by(1000) {
        raw[index / 8] |= 0x80 >> (index % 8);
    }
    raw
}

/// A bitstring in the form a status list is published in: gzipped, then base64url
/// with the multibase `u` prefix.
fn encode(raw: &[u8]) -> String {
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(raw).unwrap();
    format!(
        "u{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(gz.finish().unwrap())
    )
}

/// Decoding a published list, and testing one bit of it.
fn status_list_benchmarks(c: &mut Criterion) {
    let mut decode = c.benchmark_group("status_list_decode");
    for bits in [131_072_usize, 1_048_576] {
        let raw = bitstring(bits);
        let encoded = encode(&raw);
        decode.throughput(Throughput::Bytes(raw.len() as u64));
        decode.bench_with_input(BenchmarkId::from_parameter(bits), &encoded, |b, encoded| {
            b.iter(|| StatusList::from_encoded_list(encoded).unwrap());
        });
    }
    decode.finish();

    let list = StatusList::from_bitstring(bitstring(131_072));
    assert_eq!(list.get(0), Some(true));
    assert_eq!(list.get(1), Some(false));
    assert_eq!(list.get(1 << 20), None);
    let mut lookup = c.benchmark_group("status_list_get");
    for (label, index) in [("set", 0_usize), ("clear", 1), ("out_of_range", 1 << 20)] {
        lookup.bench_function(label, |b| b.iter(|| list.get(black_box(index))));
    }
    lookup.finish();
}

criterion_group!(benches, status_list_benchmarks);
criterion_main!(benches);
