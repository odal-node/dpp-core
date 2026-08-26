//! How a passport reference resolves and round-trips.

use super::reference::*;

#[test]
fn passport_ref_round_trips_via_camel_case_json() {
    let r = PassportRef {
        uri: "https://id.odal-node.io/dpp/0191b2c3-d4e5-7f80-9a1b-2c3d4e5f6071".to_owned(),
        public_jws_hash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            .to_owned(),
    };

    let json = serde_json::to_value(&r).unwrap();
    assert!(json.get("uri").is_some());
    // camelCase on the wire, not snake_case.
    assert!(json.get("publicJwsHash").is_some());
    assert!(json.get("public_jws_hash").is_none());

    let back: PassportRef = serde_json::from_value(json).unwrap();
    assert_eq!(back, r);
}
