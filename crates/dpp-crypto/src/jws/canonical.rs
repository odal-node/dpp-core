//! RFC 8785 JSON Canonicalization Scheme (JCS).
//!
//! Defines the **one** canonical byte form of a JSON value. This is the signing
//! contract: [`crate::jws::signer`] signs over these bytes, and every content-binding
//! verifier (`dpp_vc::local_service`) recomputes them and compares.
//! Signature equivalence therefore rests on a defined canonical form — not on
//! incidental serde behaviour (object key order, number formatting, unicode
//! escaping) that can differ between serializers and silently break
//! content-binding.
//!
//! Backed by `serde_jcs` (RFC 8785). Kept behind this single function so the
//! canonicalization implementation can be swapped without touching callers.

use serde_json::Value;

/// Largest integer magnitude every JSON number model holds exactly: 2^53 - 1,
/// the bound that RFC 8785 takes from the I-JSON profile of JSON.
const MAX_EXACT_INTEGER: u64 = (1 << 53) - 1;

/// Canonicalize a JSON value to its RFC 8785 (JCS) byte form.
///
/// Guarantees: object keys sorted by UTF-16 code unit, no insignificant
/// whitespace, numbers in the ECMAScript `Number::toString` form (so `1.0`
/// serializes as `1`), and consistent string escaping.
///
/// Integers beyond ±(2^53 − 1) are refused. The serializer writes every integer
/// as an IEEE 754 double, so two different integers can share one canonical form
/// and a signature over one would verify against the other.
pub fn canonicalize(value: &Value) -> anyhow::Result<Vec<u8>> {
    reject_inexact_integers(value)?;
    serde_jcs::to_vec(value).map_err(|e| anyhow::anyhow!("JCS canonicalization: {e}"))
}

fn reject_inexact_integers(value: &Value) -> anyhow::Result<()> {
    match value {
        Value::Number(n) => {
            let magnitude = n.as_u64().or_else(|| n.as_i64().map(i64::unsigned_abs));
            anyhow::ensure!(
                magnitude.is_none_or(|m| m <= MAX_EXACT_INTEGER),
                "integer {n} is outside ±(2^53 - 1) and cannot be canonicalized exactly"
            );
            Ok(())
        }
        Value::Array(items) => items.iter().try_for_each(reject_inexact_integers),
        Value::Object(map) => map.values().try_for_each(reject_inexact_integers),
        Value::Null | Value::Bool(_) | Value::String(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
