//! Tests of the `status_list` module.

use super::super::*;
use base64::Engine;
use flate2::{Compression, write::GzEncoder};
use std::io::Write;

fn encode_list(bits: &[u8]) -> String {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(bits).unwrap();
    let gz = enc.finish().unwrap();
    format!(
        "u{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(gz)
    )
}

#[test]
fn a_list_that_expands_past_the_budget_is_refused() {
    let encoded = encode_list(&vec![0u8; 4096]);
    let err = StatusList::from_encoded_list_within(&encoded, 1024).unwrap_err();
    assert!(err.to_string().contains("more than 1024 bytes"), "{err}");
}

#[test]
fn a_list_exactly_at_the_budget_is_accepted() {
    let encoded = encode_list(&vec![0u8; 1024]);
    let list = StatusList::from_encoded_list_within(&encoded, 1024).unwrap();
    assert_eq!(list.len_bits(), 1024 * 8);
}

/// The largest budget a caller can pass must not overflow the reader's limit.
#[test]
fn the_largest_budget_reads_the_whole_list() {
    let encoded = encode_list(&vec![0u8; 1024]);
    let list = StatusList::from_encoded_list_within(&encoded, usize::MAX).unwrap();
    assert_eq!(list.len_bits(), 1024 * 8);
}

#[test]
fn a_small_input_that_expands_past_the_default_budget_is_refused() {
    // 17 MiB of zeros compresses to a few kilobytes: exactly the shape the
    // default budget exists to refuse.
    let encoded = encode_list(&vec![0u8; MAX_DECODED_BYTES + 1]);
    assert!(encoded.len() < 64 * 1024, "fixture should compress well");
    assert!(StatusList::from_encoded_list(&encoded).is_err());
}

#[test]
fn bit_indexing_is_big_endian_within_byte() {
    let list = StatusList::from_bitstring(vec![0b1000_0001]);
    assert_eq!(list.get(0), Some(true));
    assert_eq!(list.get(1), Some(false));
    assert_eq!(list.get(7), Some(true));
    assert_eq!(list.get(8), None, "out-of-range index is None");
}

#[test]
fn encoded_list_round_trips() {
    let bits = vec![0b0000_0100, 0b0000_0000];
    let encoded = encode_list(&bits);
    let list = StatusList::from_encoded_list(&encoded).expect("decode");
    assert_eq!(list.get(5), Some(true));
    assert_eq!(list.get(4), Some(false));
    assert_eq!(list.get(6), Some(false));
    assert_eq!(list.len_bits(), 16);
}

#[test]
fn decodes_without_multibase_prefix() {
    let bits = vec![0b1000_0000];
    let encoded = encode_list(&bits);
    let without_u = encoded.strip_prefix('u').unwrap();
    let list = StatusList::from_encoded_list(without_u).expect("decode");
    assert_eq!(list.get(0), Some(true));
}

#[test]
fn garbage_is_an_error_not_a_panic() {
    assert!(StatusList::from_encoded_list("u!!!not-base64!!!").is_err());
    assert!(StatusList::from_encoded_list("udGhpcyBpcyBub3QgZ3ppcA").is_err());
}
