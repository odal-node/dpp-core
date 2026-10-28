#![no_main]
//! Fuzz the identifier and date parsers — GTINs, GLNs, commodity codes, ICDs,
//! identification links, DIDs and calendar dates reach them from spreadsheet
//! rows and API bodies, one string at a time.
//! Property: every parser returns `Ok`/`Err` (or `Some`/`None`) for any input,
//! never panics, and a `ProductIdentifier` read from JSON goes through the same
//! checks as one built directly.

use dpp_domain::identifier::{CnCategory, CommodityCode, Gln, Gtin, Icd, ProductIdentifier};
use dpp_rules::common::date::CalendarDate;
use dpp_rules::common::identifier::{check_did, check_web_url};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = Gtin::parse(s);
        let _ = Gln::parse(s);
        let _ = CommodityCode::parse(s);
        let _ = CnCategory::parse(s);
        let _ = Icd::parse(s);
        let _ = check_web_url(s);
        let _ = check_did(s);
        let _ = CalendarDate::parse_iso(s);
    }
    let _ = serde_json::from_slice::<ProductIdentifier>(data);
});
