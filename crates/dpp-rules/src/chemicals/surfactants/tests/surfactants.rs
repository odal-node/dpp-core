//! Tests of the `surfactants` module.

use super::super::*;

fn surf<'a>(name: &'a str, band: &'a str, biodegradable: bool) -> SurfactantInput<'a> {
    SurfactantInput {
        name,
        concentration_band: band,
        biodegradable,
        cas_number: None,
    }
}

#[test]
fn surfactant_bands_and_validation() {
    assert!(surfactant_band_valid("5-15%"));
    assert!(!surfactant_band_valid("lots"));

    assert!(validate_surfactants(&[surf("LAS", "5-15%", true)]).is_ok());
    assert!(validate_surfactants(&[]).is_err());
    assert!(validate_surfactants(&[surf("LAS", "lots", true)]).is_err());
}

#[test]
fn non_biodegradable_surfactant_rejected() {
    let err = validate_surfactants(&[surf("APEO", "5-15%", false)]).unwrap_err();
    assert!(err.contains("biodegradable"), "unexpected: {err}");
}

#[test]
fn biodegradable_flag_checked_per_entry() {
    let ok = surf("LAS", "5-15%", true);
    let bad = surf("APEO", "<5%", false);
    let err = validate_surfactants(&[ok, bad]).unwrap_err();
    assert!(err.contains("surfactants[1]"), "unexpected: {err}");
}

#[test]
fn cas_number_threaded_through() {
    let s = SurfactantInput {
        name: "LAS",
        concentration_band: "5-15%",
        biodegradable: true,
        cas_number: Some("25155-30-0"),
    };
    assert!(validate_surfactants(&[s]).is_ok());
}
