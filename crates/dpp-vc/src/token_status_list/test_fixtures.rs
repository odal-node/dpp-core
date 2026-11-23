//! Fixtures shared by this module's test files.

use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Value, json};

use dpp_crypto::keystore::KeyEntry;

use super::{StatusReference, TYP, TokenStatusList};
use crate::test_support::forge;

/// The `sub` of the draft's example token (section 5.1), and so the `uri` its
/// credentials refer to.
pub(super) const URI: &str = "https://example.com/statuslists/1";

/// The `iat` of the draft's example token.
pub(super) const ISSUED_AT: i64 = 1_686_920_170;

/// The `exp` of the draft's example token, far past [`now`].
pub(super) const EXPIRES_AT: i64 = 2_291_720_170;

/// A fixed instant a little after the draft's `iat`, so nothing here depends on
/// the wall clock.
pub(super) fn now() -> DateTime<Utc> {
    Utc.timestamp_opt(ISSUED_AT + 60, 0).unwrap()
}

pub(super) fn reference(idx: u64) -> StatusReference {
    StatusReference {
        idx,
        uri: URI.to_owned(),
    }
}

/// The protected header of a Status List Token signed by `key`.
pub(super) fn header(key: &KeyEntry) -> Value {
    json!({ "alg": key.algorithm().jose_alg(), "kid": "12", "typ": TYP })
}

/// The claims of the draft's example token (section 5.1) around `list`.
pub(super) fn claims(list: &TokenStatusList) -> Value {
    json!({
        "sub": URI,
        "iat": ISSUED_AT,
        "exp": EXPIRES_AT,
        "ttl": 43_200,
        "status_list": { "bits": list.bits(), "lst": list.encode() },
    })
}

/// A Status List Token over `claims`, signed by `key`.
pub(super) fn token(key: &KeyEntry, claims: &Value) -> String {
    forge(key, &header(key), claims)
}

/// A two-bit list of eight entries holding one of each status of section 7: entry 0
/// is valid, 1 revoked, 2 suspended and 3 application-specific.
pub(super) fn list() -> TokenStatusList {
    let mut list = TokenStatusList::new(2, 8).expect("a list of eight two-bit entries");
    for (idx, value) in [(1, 1), (2, 2), (3, 3)] {
        list.set(idx, value).expect("inside the list");
    }
    list
}
