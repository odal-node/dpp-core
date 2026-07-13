#![no_main]
//! Fuzz the GS1 Digital Link URI parser — the byte frontier a scanned QR reaches.
//! Property: `DigitalLink::parse` returns `Ok`/`Err` for any input, never panics.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = dpp_digital_link::DigitalLink::parse(s);
    }
});
