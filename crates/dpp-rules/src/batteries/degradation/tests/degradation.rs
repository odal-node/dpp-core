//! Tests of the `degradation` module.

use super::super::*;

#[test]
fn ev_batteries_report_soce_alone() {
    assert_eq!(
        annex_vii_parameter_set_for("ev"),
        Some(Annex7ParameterSet::ElectricVehicle)
    );
    assert_eq!(
        annex_vii_parameter_set_for("EV"),
        Some(Annex7ParameterSet::ElectricVehicle)
    );
}

#[test]
fn lmt_and_industrial_share_the_five_parameter_list() {
    for t in ["lmt", "LMT", "industrial", " Industrial "] {
        assert_eq!(
            annex_vii_parameter_set_for(t),
            Some(Annex7ParameterSet::StationaryOrLmt),
            "{t} should use the stationary/LMT list"
        );
    }
}

#[test]
fn portable_and_sli_are_outside_annex_vii_part_a() {
    // Art. 14(1) names stationary storage, LMT and EV batteries only.
    for t in [
        "portable",
        "sli",
        "starting-lighting-ignition",
        "",
        "mystery",
    ] {
        assert_eq!(
            annex_vii_parameter_set_for(t),
            None,
            "{t} should be out of scope"
        );
    }
}

#[test]
fn part_b_excludes_electric_vehicle_batteries() {
    // The asymmetry between the two parts. Part A names EV batteries;
    // Part B does not. An EV battery reports SOCE and no expected-lifetime
    // parameter set at all.
    assert_eq!(
        annex_vii_parameter_set_for("ev"),
        Some(Annex7ParameterSet::ElectricVehicle)
    );
    assert!(!annex_vii_part_b_applies_to("ev"));
}

#[test]
fn part_b_covers_stationary_and_lmt() {
    for t in ["lmt", "industrial", "LMT"] {
        assert!(annex_vii_part_b_applies_to(t), "{t} should be in Part B");
    }
    for t in ["portable", "sli", "starting-lighting-ignition", "mystery"] {
        assert!(
            !annex_vii_part_b_applies_to(t),
            "{t} should be outside Part B"
        );
    }
}

#[test]
fn the_two_parameter_sets_are_distinct() {
    assert_ne!(
        annex_vii_parameter_set_for("ev"),
        annex_vii_parameter_set_for("lmt")
    );
}
