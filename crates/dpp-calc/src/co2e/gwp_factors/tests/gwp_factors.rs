//! Tests of the `gwp_factors` module.

use super::super::*;

#[test]
fn co2_factor_is_one() {
    assert_eq!(by_name("CO2"), Some(1.0));
}

#[test]
fn methane_fossil_ar6() {
    assert_eq!(by_name("CH4-fossil"), Some(29.8));
}

/// CH4-biogenic GWP100 = 27.9 per EF 3.1 / AR6 WG-I Table 7.SM.7.
/// EF 3.1 uses 27.9 (not 27.0) because it attributes the CO₂ from CH4
/// oxidation back to the biogenic emission source. Changing this value
/// requires a cited update to the EF 3.1 characterisation table.
#[test]
fn methane_biogenic_ef31() {
    assert_eq!(by_name("CH4-biogenic"), Some(27.9));
    assert_eq!(by_name("methane biogenic"), Some(27.9));
}

#[test]
fn n2o_ar6() {
    assert_eq!(by_name("N2O"), Some(273.0));
}

/// SF₆ GWP100 = 25_200 per IPCC AR6 WG-I Table 7.SM.7 (no Earth-system
/// feedbacks), as adopted by EF 3.1. The previous AR5 value was 23_500 —
/// a ~7 % underestimate that materially affects electronics and semiconductor
/// passports. Any change here must cite the updated IPCC/EF source table.
#[test]
fn sf6_is_ar6_value_not_ar5() {
    let gwp = by_name("SF6").expect("SF6 must be in the table");
    assert!(
        (gwp - 25_200.0).abs() < 1.0,
        "SF6 GWP100 must be 25_200 (AR6 WG-I Table 7.SM.7), got {gwp}"
    );
    // Explicitly reject the old AR5 value so a silent revert is caught.
    assert!(
        (gwp - 23_500.0).abs() > 1.0,
        "SF6 must not be the AR5 value 23_500"
    );
    // Both aliases resolve to the same value.
    assert_eq!(by_name("sulphur hexafluoride"), Some(25_200.0));
}

#[test]
fn hfc_and_pfc_spot_check() {
    assert_eq!(by_name("HFC-134a"), Some(1_526.0));
    assert_eq!(by_name("HFC-32"), Some(771.0));
    assert_eq!(by_name("PFC-14"), Some(7_380.0));
    assert_eq!(by_name("CF4"), Some(7_380.0));
}

#[test]
fn unknown_returns_none() {
    assert_eq!(by_name("unobtanium"), None);
}
