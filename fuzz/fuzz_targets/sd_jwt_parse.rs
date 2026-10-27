#![no_main]
//! Fuzz SD-JWT parsing — a holder presents a selectively disclosed passport as
//! an issuer-signed JWT followed by `~`-separated disclosures, and every part of
//! that string is under the holder's control.
//! Property: `SdJwt::parse` and `Disclosure::parse` return `Ok`/`Err` for any
//! input, never panic.

use dpp_crypto::sd_jwt::{Disclosure, SdJwt};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = SdJwt::parse(s);
        let _ = Disclosure::parse(s);
    }
});
