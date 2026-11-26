//! Tests of the `canonical` module.

use super::super::*;
use serde_json::json;

#[test]
fn the_largest_exact_integer_is_canonicalized_as_written() {
    let out = canonicalize(&json!({"n": 9_007_199_254_740_991_u64})).unwrap();
    assert_eq!(out, br#"{"n":9007199254740991}"#);
}

#[test]
fn an_integer_that_a_double_cannot_hold_is_refused() {
    // 2^53 + 1 is written as 2^53 by an f64; a signature over one must not
    // verify against the other.
    assert!(canonicalize(&json!(9_007_199_254_740_993_u64)).is_err());
    assert!(canonicalize(&json!(-9_007_199_254_740_993_i64)).is_err());
}

#[test]
fn an_inexact_integer_nested_anywhere_is_refused() {
    assert!(canonicalize(&json!({"a": [{"b": 9_007_199_254_740_993_u64}]})).is_err());
}

#[test]
fn object_keys_are_sorted_and_compact() {
    let out = canonicalize(&json!({"b": 1, "a": 2, "c": 3})).unwrap();
    assert_eq!(out, br#"{"a":2,"b":1,"c":3}"#);
}

#[test]
fn nested_object_keys_are_sorted() {
    let out = canonicalize(&json!({"z": {"b": 1, "a": 2}, "a": 0})).unwrap();
    assert_eq!(out, br#"{"a":0,"z":{"a":2,"b":1}}"#);
}

#[test]
fn key_order_is_irrelevant_to_output() {
    let a = canonicalize(&json!({"one": 1, "two": 2})).unwrap();
    let b = canonicalize(&json!({"two": 2, "one": 1})).unwrap();
    assert_eq!(a, b, "canonical form must not depend on source key order");
}

#[test]
fn integer_valued_float_is_normalized() {
    assert_eq!(canonicalize(&json!(1.0)).unwrap(), b"1");
    assert_eq!(canonicalize(&json!({"v": 3.0})).unwrap(), br#"{"v":3}"#);
}

#[test]
fn no_insignificant_whitespace() {
    let out = canonicalize(&json!({"a": [1, 2, 3], "b": "x"})).unwrap();
    let s = String::from_utf8(out).unwrap();
    assert!(
        !s.contains(' ') && !s.contains('\n'),
        "JCS output has no insignificant whitespace: {s}"
    );
}
