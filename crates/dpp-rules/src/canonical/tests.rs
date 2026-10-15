use super::content_hash;
use serde_json::json;

/// The property every caller relies on: canonicalisation is key-order
/// independent, so a signer and a verifier that built the same value by
/// different routes still agree on the digest.
#[test]
fn hash_is_key_order_independent() {
    let a = json!({ "alpha": 1, "beta": { "x": true, "y": [1, 2] } });
    let b = json!({ "beta": { "y": [1, 2], "x": true }, "alpha": 1 });
    assert_eq!(content_hash(&a).unwrap(), content_hash(&b).unwrap());
}

/// Array order, unlike key order, is semantic and must change the digest.
#[test]
fn hash_is_array_order_sensitive() {
    let a = json!({ "items": [1, 2] });
    let b = json!({ "items": [2, 1] });
    assert_ne!(content_hash(&a).unwrap(), content_hash(&b).unwrap());
}

/// A changed value changes the digest — the tamper-detection property.
#[test]
fn hash_detects_value_change() {
    let a = json!({ "threshold": 10 });
    let b = json!({ "threshold": 11 });
    assert_ne!(content_hash(&a).unwrap(), content_hash(&b).unwrap());
}

/// A typed value hashes to exactly what its `Value` form does, so a caller
/// may hand over either and land on the same digest.
#[test]
fn typed_and_value_forms_hash_alike() {
    let typed = ("threshold", [1.5_f64, 2.0]);
    assert_eq!(
        content_hash(&typed).unwrap(),
        content_hash(&json!(["threshold", [1.5, 2.0]])).unwrap()
    );
}

/// Why the hasher takes any `Serialize`: going through `Value` first would
/// quietly hash `null` where the caller held a non-finite float, while hashing
/// the float itself refuses it.
#[test]
fn a_non_finite_float_is_refused_not_hashed_as_null() {
    assert!(content_hash(&f64::NAN).is_err());
    assert_eq!(
        serde_json::to_value(f64::NAN).unwrap(),
        serde_json::Value::Null,
        "if this stops holding, the reason in content_hash's docs is stale"
    );
}
