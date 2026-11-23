#![no_main]
//! Fuzz Token Status List decoding — a credential's list is fetched from the status
//! issuer's URL, and its `lst` is base64url text that inflates with zlib, so the
//! encoding, the width and the compressed size are the status issuer's to choose. The
//! reference a credential carries is whatever its issuer wrote.
//! Property: `TokenStatusList::from_encoded_within` returns `Ok`/`Err` for any text and
//! any width, never panics; a decoded list answers `get` at any index, and writing an
//! entry back as it was read leaves the list as it was; `StatusReference::from_claims`
//! returns `Ok`/`Err` for any JSON object.

use dpp_vc::token_status_list::{Status, StatusReference, TokenStatusList};
use libfuzzer_sys::fuzz_target;

/// What a list may inflate to here: a megabyte, so that an input that asks for the
/// most still keeps an iteration fast. The default budget is the unit tests' to hold.
const BUDGET: usize = 1 << 20;

fuzz_target!(|data: &[u8]| {
    let Some((first, rest)) = data.split_first() else {
        return;
    };
    // The width is the first byte's choice: the four the draft allows, and one it
    // does not, which must be refused and not read.
    let bits = [1u8, 2, 4, 8, 3][usize::from(*first) % 5];

    if let Ok(text) = std::str::from_utf8(rest)
        && let Ok(mut list) = TokenStatusList::from_encoded_within(bits, text, BUDGET)
    {
        for idx in [0, 1, 7, 8, 131_071, u64::MAX] {
            if let Some(value) = list.get(idx) {
                let _ = Status::from_value(value);
                let before = list.clone();
                assert!(
                    list.set(idx, value).is_ok(),
                    "an entry read does not fit back"
                );
                assert_eq!(list, before, "writing an entry back changed the list");
            }
            let _ = list.set(idx, u8::MAX);
        }
    }

    if let Ok(serde_json::Value::Object(claims)) = serde_json::from_slice(rest) {
        let _ = StatusReference::from_claims(&claims);
    }
});
