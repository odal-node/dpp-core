//! Which acts set a condition on customer personal data, and how a delegated act
//! inherits its framework's.

use super::{InstrumentCatalog, PersonalDataStorage};
use crate::personal_data::LawfulBasis;

/// The three acts that say it, read against their Official Journal texts:
/// Regulation (EU) 2024/1781 Art. 10(1)(e), Regulation (EU) 2025/2509 Art.
/// 20(10) and Regulation (EU) 2026/405 Art. 22(h).
#[test]
fn the_three_acts_that_set_a_condition_each_carry_it() {
    let catalog = InstrumentCatalog::new();
    for (id, provision) in [
        ("espr", "Regulation (EU) 2024/1781 Art. 10(1)(e)"),
        (
            "toy-safety-2025-2509",
            "Regulation (EU) 2025/2509 Art. 20(10)",
        ),
        ("detergents-2026-405", "Regulation (EU) 2026/405 Art. 22(h)"),
    ] {
        let condition = catalog
            .customer_personal_data_for(id)
            .unwrap_or_else(|| panic!("{id} sets a condition"));
        assert_eq!(condition.provision, provision);
        assert_eq!(condition.storage, PersonalDataStorage::ExplicitConsentOnly);
    }
}

/// The Batteries Regulation has no personal-data provision; its passport
/// answers to GDPR alone.
#[test]
fn the_batteries_regulation_sets_none() {
    let catalog = InstrumentCatalog::new();
    assert!(
        catalog
            .customer_personal_data_for("battery-reg-2023-1542")
            .is_none()
    );
}

/// ESPR's essential requirements are what its delegated and implementing acts
/// implement, so each carries ESPR's condition without restating it.
#[test]
fn an_act_under_espr_inherits_its_condition() {
    let catalog = InstrumentCatalog::new();
    let espr = catalog.customer_personal_data_for("espr");
    for child in catalog
        .all()
        .iter()
        .filter(|i| i.parent.as_deref() == Some("espr"))
    {
        assert_eq!(
            catalog.customer_personal_data_for(&child.id),
            espr,
            "{} sits under ESPR and must carry its condition",
            child.id
        );
    }
}

#[test]
fn an_act_the_catalog_does_not_hold_has_no_condition_to_report() {
    assert!(
        InstrumentCatalog::new()
            .customer_personal_data_for("no-such-act")
            .is_none()
    );
}

/// A provision is a citation, and a citation must point at the act it is
/// recorded on: a condition copied between manifests would otherwise cite the
/// wrong regulation and still pass every other test.
#[test]
fn every_condition_cites_its_own_act() {
    let catalog = InstrumentCatalog::new();
    for instrument in catalog.all() {
        let Some(condition) = &instrument.customer_personal_data else {
            continue;
        };
        let celex = instrument
            .celex
            .as_deref()
            .unwrap_or_else(|| panic!("{} sets a condition but has no text", instrument.id));
        // `32024R1781` → `2024/1781`.
        let number = format!("{}/{}", &celex[1..5], celex[6..].trim_start_matches('0'));
        assert!(
            condition.provision.contains(&number),
            "{}: provision {:?} does not cite {number}",
            instrument.id,
            condition.provision
        );
    }
}

#[test]
fn explicit_consent_admits_consent_and_nothing_else() {
    let condition = InstrumentCatalog::new()
        .customer_personal_data_for("espr")
        .cloned()
        .unwrap();
    assert!(condition.admits(LawfulBasis::Consent));
    for other in [
        LawfulBasis::Contract,
        LawfulBasis::LegalObligation,
        LawfulBasis::VitalInterests,
        LawfulBasis::PublicTask,
        LawfulBasis::LegitimateInterests,
    ] {
        assert!(!condition.admits(other), "{other:?}");
    }
}
