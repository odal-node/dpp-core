//! RFC 8785's own examples, checked against the canonicaliser every signature
//! and content hash here is made over.
//!
//! The tests beside `canonical.rs` compare [`canonicalize`] with what its author
//! expected, and the expectation was written next to the code. A shared
//! misreading would pass all of them: a number formatted a little differently, or
//! keys ordered by UTF-8 bytes instead of UTF-16 code units. The result would be
//! signatures over bytes that a conforming verifier does not reproduce. These
//! compare against values the RFC publishes.
//!
//! Three sets:
//!
//! - Appendix B's table of number serialisations, edge cases included;
//! - the worked example of clauses 3.2.2 to 3.2.4, with the bytes the RFC prints;
//! - clause 3.2.3's data for the property sort, which orders by UTF-16 code unit.
//!
//! The two JSON inputs keep the RFC's `\u` escapes, because the escapes are part
//! of what the examples test: a parser must decode them, and the canonical form
//! must then write the characters back as the RFC says.
//!
//! # What is not here
//!
//! Appendix B's NaN and Infinity rows, which the RFC says must be an error.
//! [`canonicalize`] takes a [`Value`], and a `Value` cannot hold either, so
//! neither can reach it. [`non_finite_numbers_cannot_reach_the_canonicaliser`]
//! pins that premise. The RFC's larger number file is not run.

use serde_json::{Number, Value};

use super::canonical::canonicalize;

/// Appendix B, Table 1: the IEEE 754 bit pattern and the text it serialises to.
/// Every row of the table that has a text, in the RFC's order.
const NUMBERS: [(&str, &str); 24] = [
    ("0000000000000000", "0"),
    ("8000000000000000", "0"),
    ("0000000000000001", "5e-324"),
    ("8000000000000001", "-5e-324"),
    ("7fefffffffffffff", "1.7976931348623157e+308"),
    ("ffefffffffffffff", "-1.7976931348623157e+308"),
    ("4340000000000000", "9007199254740992"),
    ("c340000000000000", "-9007199254740992"),
    ("4430000000000000", "295147905179352830000"),
    ("44b52d02c7e14af5", "9.999999999999997e+22"),
    ("44b52d02c7e14af6", "1e+23"),
    ("44b52d02c7e14af7", "1.0000000000000001e+23"),
    ("444b1ae4d6e2ef4e", "999999999999999700000"),
    ("444b1ae4d6e2ef4f", "999999999999999900000"),
    ("444b1ae4d6e2ef50", "1e+21"),
    ("3eb0c6f7a0b5ed8c", "9.999999999999997e-7"),
    ("3eb0c6f7a0b5ed8d", "0.000001"),
    ("41b3de4355555553", "333333333.3333332"),
    ("41b3de4355555554", "333333333.33333325"),
    ("41b3de4355555555", "333333333.3333333"),
    ("41b3de4355555556", "333333333.3333334"),
    ("41b3de4355555557", "333333333.33333343"),
    ("becbf647612f3696", "-0.0000033333333333333333"),
    ("43143ff3c1cb0959", "1424953923781206.2"),
];

/// The JSON of clause 3.2.2, as the RFC prints it, before any serialiser has
/// touched it: exponents, trailing zeros, escapes and a solidus escape.
const EXAMPLE_INPUT: &str = r#"{
  "numbers": [333333333.33333329, 1E30, 4.50,
              2e-3, 0.000000000000000000000000001],
  "string": "\u20ac$\u000F\u000aA'\u0042\u0022\u005c\\\"\/",
  "literals": [null, true, false]
}"#;

/// Clause 3.2.4's hexadecimal dump of the canonical form of that example.
const EXAMPLE_CANONICAL_HEX: &str = "\
7b 22 6c 69 74 65 72 61 6c 73 22 3a 5b 6e 75 6c 6c 2c 74 72 \
75 65 2c 66 61 6c 73 65 5d 2c 22 6e 75 6d 62 65 72 73 22 3a \
5b 33 33 33 33 33 33 33 33 33 2e 33 33 33 33 33 33 33 2c 31 \
65 2b 33 30 2c 34 2e 35 2c 30 2e 30 30 32 2c 31 65 2d 32 37 \
5d 2c 22 73 74 72 69 6e 67 22 3a 22 e2 82 ac 24 5c 75 30 30 \
30 66 5c 6e 41 27 42 5c 22 5c 5c 5c 5c 5c 22 2f 22 7d";

/// Clause 3.2.3's data for the property sort.
const SORT_INPUT: &str = r#"{
  "\u20ac": "Euro Sign",
  "\r": "Carriage Return",
  "\ufb33": "Hebrew Letter Dalet With Dagesh",
  "1": "One",
  "\ud83d\ude00": "Emoji: Grinning Face",
  "\u0080": "Control",
  "\u00f6": "Latin Small Letter O With Diaeresis"
}"#;

/// The same clause's expected order of the values after sorting their keys.
const SORT_ORDER: [&str; 7] = [
    "Carriage Return",
    "One",
    "Control",
    "Latin Small Letter O With Diaeresis",
    "Euro Sign",
    "Emoji: Grinning Face",
    "Hebrew Letter Dalet With Dagesh",
];

#[test]
fn every_appendix_b_number_serialises_as_the_rfc_prints_it() {
    for (bits, expected) in NUMBERS {
        let bits = u64::from_str_radix(bits, 16).expect("the table holds 64-bit hex");
        let number = Number::from_f64(f64::from_bits(bits)).expect("a finite value");
        let canonical = canonicalize(&Value::Number(number)).expect("canonicalises");
        assert_eq!(
            String::from_utf8(canonical).expect("UTF-8"),
            expected,
            "IEEE 754 {bits:016x}"
        );
    }
}

/// The RFC publishes the exact bytes. Comparing bytes and not text is the point:
/// the escapes, the euro sign's three bytes, and the number forms are all in it.
#[test]
fn the_worked_example_canonicalises_to_the_rfc_bytes() {
    let value: Value = serde_json::from_str(EXAMPLE_INPUT).expect("the RFC's JSON parses");
    let expected = hex::decode(EXAMPLE_CANONICAL_HEX.replace(' ', "")).expect("hex dump");

    assert_eq!(canonicalize(&value).expect("canonicalises"), expected);
}

/// Clause 3.2.3 prints the same output as text. Pinned beside the bytes so the
/// two readings of the clause agree with each other.
#[test]
fn the_worked_example_reads_as_the_rfc_prints_it() {
    let value: Value = serde_json::from_str(EXAMPLE_INPUT).expect("the RFC's JSON parses");
    let canonical = String::from_utf8(canonicalize(&value).expect("canonicalises")).expect("UTF-8");

    assert_eq!(
        canonical,
        r#"{"literals":[null,true,false],"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27],"string":"€$\u000f\nA'B\"\\\\\"/"}"#
    );
}

/// UTF-16 code unit order, not UTF-8 byte order: the emoji (a surrogate pair,
/// `d83d` `de00`) sorts before U+FB33 although its UTF-8 encoding sorts after.
#[test]
fn properties_sort_by_utf16_code_unit() {
    let value: Value = serde_json::from_str(SORT_INPUT).expect("the RFC's JSON parses");
    let canonical = String::from_utf8(canonicalize(&value).expect("canonicalises")).expect("UTF-8");

    let positions: Vec<usize> = SORT_ORDER
        .iter()
        .map(|v| {
            canonical
                .find(&format!("\"{v}\""))
                .unwrap_or_else(|| panic!("{v} is in the output"))
        })
        .collect();
    let mut ascending = positions.clone();
    ascending.sort_unstable();
    assert_eq!(positions, ascending, "{canonical}");
}

#[test]
fn non_finite_numbers_cannot_reach_the_canonicaliser() {
    assert!(Number::from_f64(f64::NAN).is_none());
    assert!(Number::from_f64(f64::INFINITY).is_none());
    assert!(Number::from_f64(f64::NEG_INFINITY).is_none());
}
