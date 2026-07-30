//! JSON-LD frame/strip round-trip tests.

use serde_json::json;

use super::*;

#[test]
fn frame_and_strip_round_trip() {
    let passport = json!({ "passportId": "abc", "sector": "battery" });
    let framed = frame_passport(passport.clone());
    let stripped = strip_context(framed);
    assert_eq!(stripped["passportId"], "abc");
    assert!(stripped.get("@context").is_none());
}

#[test]
fn frame_passport_preserves_non_object_payload() {
    // A non-object payload can't be merged into the context map, but it must be
    // returned intact rather than silently discarded into a bare envelope.
    let framed = frame_passport(json!("not-an-object"));
    assert_eq!(framed, json!("not-an-object"));
}

#[test]
fn strip_context_passes_through_non_object() {
    let array = json!(["a", "b"]);
    assert_eq!(strip_context(array.clone()), array);
}
