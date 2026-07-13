//! Property tests for the GS1 Digital Link parser.
//!
//! `DigitalLink::parse` is a hostile-input frontier (it parses resolver URIs that
//! may come from a scanned QR). These properties assert it never panics on
//! arbitrary text, and that a canonical URI built from valid components round-trips.

use dpp_digital_link::DigitalLink;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// Whatever text it is handed, the parser returns `Ok`/`Err` — never panics.
    #[test]
    fn parse_never_panics(s in ".*") {
        let _ = DigitalLink::parse(&s);
    }

    /// URL-shaped hostile input reaches deeper code paths than random noise.
    #[test]
    fn parse_never_panics_on_urlish(s in "https://[a-zA-Z0-9./%:_-]{0,60}") {
        let _ = DigitalLink::parse(&s);
    }

    /// A canonical `/01/<gtin>/21/<serial>` URI round-trips: parse recovers the
    /// same GTIN and serial. (Fixed valid GTIN; the serial is the varied part.)
    #[test]
    fn serial_round_trips(serial in "[A-Za-z0-9]{1,24}") {
        let uri = format!("https://id.odal-node.io/01/09506000134352/21/{serial}");
        let dl = DigitalLink::parse(&uri).expect("canonical DL must parse");
        prop_assert_eq!(dl.gtin.as_str(), "09506000134352");
        prop_assert_eq!(dl.serial.as_deref(), Some(serial.as_str()));
    }
}
