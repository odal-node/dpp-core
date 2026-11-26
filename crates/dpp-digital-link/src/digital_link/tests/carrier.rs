//! The carrier qualifier a link names.

use super::super::*;

/// A link names the passport-carrier qualifier its path carries after the
/// GTIN: nothing is the model, a lot is a batch, and a serial is one unit —
/// with or without a lot before it, which earlier carriers printed.
#[test]
fn a_link_names_the_carrier_qualifier_its_path_carries() {
    use dpp_domain::CarrierQualifier;
    use std::borrow::Cow;

    let base = "https://id.example.com/01/09506000134352";
    for (path, expected) in [
        ("", CarrierQualifier::Model),
        (
            "/10/LOT%2FA",
            CarrierQualifier::Batch(Cow::Borrowed("LOT/A")),
        ),
        ("/21/SN1", CarrierQualifier::Serial(Cow::Borrowed("SN1"))),
        (
            "/10/LOT1/21/SN1",
            CarrierQualifier::Serial(Cow::Borrowed("SN1")),
        ),
    ] {
        let link = DigitalLink::parse(&format!("{base}{path}")).unwrap();
        assert_eq!(link.carrier_qualifier(), Some(expected), "{path}");
    }
}

/// No passport carrier prints AI 22 or AI 235, or keys on anything but a GTIN,
/// so such a link names no carrier qualifier — dropping what it carries would
/// resolve it to a different thing than it names.
#[test]
fn a_link_no_carrier_prints_names_no_carrier_qualifier() {
    for uri in [
        "https://id.example.com/01/09506000134352/22/VAR1",
        "https://id.example.com/01/09506000134352/22/VAR1/21/SN1",
        "https://id.example.com/01/09506000134352/235/TPX1",
        "https://id.example.com/00/106141411234567897",
    ] {
        let link = DigitalLink::parse(uri).unwrap();
        assert_eq!(link.carrier_qualifier(), None, "{uri}");
    }
}
