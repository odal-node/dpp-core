#![no_main]
//! Fuzz Bitstring Status List decoding — a credential's revocation list is
//! fetched from the issuer's URL as base64url text that inflates with gzip, so
//! both the encoding and the compressed size are the issuer's to choose.
//! Property: `StatusList::from_encoded_list` returns `Ok`/`Err` for any input,
//! never panics, and a decoded list answers `get` at any index.

use dpp_vc::status_list::StatusList;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(encoded) = std::str::from_utf8(data) {
        if let Ok(list) = StatusList::from_encoded_list(encoded) {
            for index in [0, 1, 7, 8, 131_071, usize::MAX] {
                let _ = list.get(index);
            }
        }
    }
});
