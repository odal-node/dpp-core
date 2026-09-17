//! Art. 1 of Reg. (EU) 2023/1669 in both directions: what it reaches, what it
//! carves out, and what a missing declaration may not buy.

use super::repairability_index_scope::{RepairabilityIndexScope, repairability_index_scope};

#[test]
fn the_act_reaches_smartphones_and_slate_tablets_only() {
    for t in ["smartphone", "tablet"] {
        assert_eq!(
            repairability_index_scope(t, None),
            RepairabilityIndexScope::Covered,
            "Art. 1 names {t}"
        );
    }
    // In scope of 2023/1670, which is a different act with a wider scope.
    for t in ["other-mobile-phone", "cordless-phone"] {
        assert_eq!(
            repairability_index_scope(t, None),
            RepairabilityIndexScope::NotCovered,
            "{t} is outside 2023/1669"
        );
    }
}

#[test]
fn both_article_1_exclusions_are_expressible() {
    assert_eq!(
        repairability_index_scope("smartphone", Some("rollable-display")),
        RepairabilityIndexScope::ExcludedRollableDisplay
    );
    assert_eq!(
        repairability_index_scope("smartphone", Some("high-security-communication")),
        RepairabilityIndexScope::ExcludedHighSecurity
    );
    // Art. 1(a) says "mobile phones **and tablets**", so it reaches a tablet too.
    assert_eq!(
        repairability_index_scope("tablet", Some("rollable-display")),
        RepairabilityIndexScope::ExcludedRollableDisplay
    );
}

#[test]
fn an_undeclared_exclusion_is_not_an_exclusion() {
    // The carve-out is what removes the obligation, so silence cannot grant it.
    assert_eq!(
        repairability_index_scope("smartphone", None),
        RepairabilityIndexScope::Covered
    );
    // Nor may a value this build does not recognise. A string we cannot read is
    // not a claim we may act on, and treating it as one would let any typo
    // exempt a product.
    for unknown in ["", "  ", "flexible", "rollable_display", "secure"] {
        assert_eq!(
            repairability_index_scope("smartphone", Some(unknown)),
            RepairabilityIndexScope::Covered,
            "{unknown:?} must not exempt"
        );
    }
}

#[test]
fn the_device_type_is_decided_before_the_exclusion() {
    // An exclusion from an act that never reached the product is not the reason
    // to give an operator: the answer is that 2023/1669 does not cover it.
    assert_eq!(
        repairability_index_scope("cordless-phone", Some("rollable-display")),
        RepairabilityIndexScope::NotCovered
    );
}

#[test]
fn matching_tolerates_case_and_surrounding_space() {
    assert_eq!(
        repairability_index_scope("  SmartPhone  ", None),
        RepairabilityIndexScope::Covered
    );
    assert_eq!(
        repairability_index_scope("smartphone", Some(" High-Security-Communication ")),
        RepairabilityIndexScope::ExcludedHighSecurity
    );
}

#[test]
fn the_two_helpers_agree_with_the_variants() {
    assert!(RepairabilityIndexScope::Covered.is_covered());
    assert!(!RepairabilityIndexScope::Covered.is_declared_exclusion());

    for excluded in [
        RepairabilityIndexScope::ExcludedRollableDisplay,
        RepairabilityIndexScope::ExcludedHighSecurity,
    ] {
        assert!(!excluded.is_covered());
        assert!(
            excluded.is_declared_exclusion(),
            "an operator said so, and a claim can be wrong"
        );
    }

    // NotCovered is not a declared exclusion — nobody claimed anything, the act
    // simply does not reach the device type.
    assert!(!RepairabilityIndexScope::NotCovered.is_covered());
    assert!(!RepairabilityIndexScope::NotCovered.is_declared_exclusion());
}
