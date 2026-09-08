//! What the passport's operator snapshot carries, and what it checks.

use super::{OperatorRole, ResponsibilityBasis, ResponsibleOperator, ResponsibleOperatorSnapshot};

fn snapshot(role: OperatorRole, basis: ResponsibilityBasis) -> ResponsibleOperatorSnapshot {
    ResponsibleOperatorSnapshot {
        operator: ResponsibleOperator {
            did: "did:web:acme.example.com".into(),
            name: "Acme GmbH".into(),
            role,
            eu_operator_id: Some("DE123456789".into()),
            eu_operator_id_scheme: Some("vat".into()),
            country: "DE".into(),
            registered_trade_name: Some("Acme".into()),
            postal_address: Some("Alexanderplatz 1, 10178 Berlin".into()),
            electronic_address: Some("compliance@acme.example.com".into()),
        },
        basis,
    }
}

/// All three of Annex III(k)'s elements are reachable from one value: the name,
/// the contact details, and the unique operator identifier.
#[test]
fn the_snapshot_carries_all_three_annex_iii_k_elements() {
    let s = snapshot(
        OperatorRole::AuthorisedRepresentative,
        ResponsibilityBasis::MarketSurveillanceArt4,
    );
    assert_eq!(s.operator.name, "Acme GmbH");
    assert!(s.operator.postal_address.is_some());
    assert!(s.operator.electronic_address.is_some());
    assert_eq!(s.operator.eu_operator_id.as_deref(), Some("DE123456789"));
}

/// Where Art. 4 is the basis, the role has to be one of the four the article
/// names. This is the check that a distributor cannot be presented as the
/// answerable party under a provision that does not admit one.
#[test]
fn a_distributor_does_not_fit_the_art_4_basis() {
    let s = snapshot(
        OperatorRole::Distributor,
        ResponsibilityBasis::MarketSurveillanceArt4,
    );
    assert!(!s.role_fits_basis());
}

#[test]
fn the_four_art_4_roles_fit_the_art_4_basis() {
    for role in [
        OperatorRole::Manufacturer,
        OperatorRole::Importer,
        OperatorRole::AuthorisedRepresentative,
        OperatorRole::FulfilmentServiceProvider,
    ] {
        let s = snapshot(role.clone(), ResponsibilityBasis::MarketSurveillanceArt4);
        assert!(s.role_fits_basis(), "{role:?} is named by Art. 4(2)");
    }
}

/// Only the Art. 4 limb has a closed role set. A general product-safety
/// responsible person is not narrowed by Art. 4(2) here, so the check must not
/// invent a constraint the regulation does not impose.
#[test]
fn another_basis_does_not_inherit_art_4s_role_set() {
    let s = snapshot(
        OperatorRole::Distributor,
        ResponsibilityBasis::GeneralProductSafety,
    );
    assert!(s.role_fits_basis());

    let s = snapshot(
        OperatorRole::Recycler,
        ResponsibilityBasis::OtherUnionLaw {
            citation: "Article 7 of Regulation (EU) 2017/745".into(),
        },
    );
    assert!(s.role_fits_basis());
}

#[test]
fn the_snapshot_round_trips() {
    let s = snapshot(
        OperatorRole::Importer,
        ResponsibilityBasis::GeneralProductSafety,
    );
    let json = serde_json::to_value(&s).unwrap();
    assert_eq!(json["operator"]["role"], "importer");
    assert_eq!(json["basis"], "generalProductSafety");
    let back: ResponsibleOperatorSnapshot = serde_json::from_value(json).unwrap();
    assert_eq!(back, s);
}
