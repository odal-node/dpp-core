//! [`PassportRef`] — a cross-operator reference to another passport.

use serde::{Deserialize, Serialize};

/// A cross-operator reference to another passport: where to fetch it, and the
/// hash that pins the exact signed public view expected there.
///
/// Direction-neutral — the owning field name carries the meaning: a
/// `parent_passport_ref` points up to the predecessor a second-life passport
/// derives from, while component references point down to constituents.
///
/// This is pure data. Computing and verifying `public_jws_hash` (a network
/// fetch plus a JWS check) is the responsibility of the stateful engine, not of
/// this crate — the referenced passport is fetched, its `public_jws_signature`
/// re-hashed, and the result compared against the value stored here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportRef {
    /// Resolvable `https` URI of the referenced passport (resolver or public route).
    pub uri: String,
    /// Lowercase hex SHA-256 over the referenced passport's compact
    /// `public_jws_signature` (UTF-8 bytes). Pins the exact signed public view:
    /// any tamper on the target changes its signature, and thus this hash.
    pub public_jws_hash: String,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
