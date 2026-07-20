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
