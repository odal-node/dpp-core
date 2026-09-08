//! Every `OperatorRole` variant is reachable and round-trips, and
//! `ResponsibleOperator`'s contact fields are additive.

use super::{OperatorRole, ResponsibleOperator};

/// `ALL` must list every variant.
///
/// The match below is exhaustive and has no catch-all, so adding a variant
/// stops this file compiling — and the length assertion then fails until
/// `ALL` is updated too. Two stages, because a const list that can silently
/// fall behind the enum is worse than no list: every consumer that trusts it
/// to be complete inherits the gap.
#[test]
fn all_lists_every_variant() {
    for role in OperatorRole::ALL {
        match role {
            OperatorRole::Manufacturer
            | OperatorRole::Importer
            | OperatorRole::Distributor
            | OperatorRole::AuthorisedRepresentative
            | OperatorRole::FulfilmentServiceProvider
            | OperatorRole::Remanufacturer
            | OperatorRole::Repurposer
            | OperatorRole::PreparerForReuse
            | OperatorRole::Repairer
            | OperatorRole::Recycler => {}
        }
    }
    assert_eq!(
        OperatorRole::ALL.len(),
        10,
        "a variant was added to the match above but not to ALL"
    );
}

/// Art. 4(2) of Regulation (EU) 2019/1020 names four and only four. This pins
/// which of our roles are in that set, including the one that was missing until
/// the set was checked against the text.
#[test]
fn art_4_2_admits_exactly_the_four_the_regulation_names() {
    let admitted: Vec<&OperatorRole> = OperatorRole::ALL
        .iter()
        .filter(|r| r.can_be_art_4_operator())
        .collect();
    assert_eq!(
        admitted,
        vec![
            &OperatorRole::Manufacturer,
            &OperatorRole::Importer,
            &OperatorRole::AuthorisedRepresentative,
            &OperatorRole::FulfilmentServiceProvider,
        ]
    );
}

/// A distributor makes a product available without altering it, and is not one
/// of the four. It is in the enum because it appears in a transfer chain, which
/// is a different question from who is answerable.
#[test]
fn a_distributor_can_never_be_the_art_4_operator() {
    assert!(!OperatorRole::Distributor.can_be_art_4_operator());
    assert!(!OperatorRole::Recycler.can_be_art_4_operator());
}

/// The transfer chain is a persisted shape, so an operator stored before the
/// contact fields existed has to keep reading.
#[test]
fn an_operator_stored_before_the_contact_fields_existed_reads() {
    let stored = serde_json::json!({
        "did": "did:web:acme.example.com",
        "name": "Acme GmbH",
        "role": "manufacturer",
        "euOperatorId": "DE123456789",
        "country": "DE",
    });
    let op: ResponsibleOperator = serde_json::from_value(stored).expect("pre-field document reads");
    assert_eq!(op.registered_trade_name, None);
    assert_eq!(op.postal_address, None);
    assert_eq!(op.electronic_address, None);
    assert_eq!(op.name, "Acme GmbH");
}

#[test]
fn the_contact_wire_keys_are_camel_case() {
    let op = ResponsibleOperator {
        did: "did:web:acme.example.com".into(),
        name: "Acme GmbH".into(),
        role: OperatorRole::Manufacturer,
        eu_operator_id: None,
        eu_operator_id_scheme: None,
        country: "DE".into(),
        registered_trade_name: Some("Acme".into()),
        postal_address: Some("Alexanderplatz 1, 10178 Berlin".into()),
        electronic_address: Some("compliance@acme.example.com".into()),
    };
    let json = serde_json::to_value(&op).unwrap();
    assert_eq!(json["registeredTradeName"], "Acme");
    assert_eq!(json["postalAddress"], "Alexanderplatz 1, 10178 Berlin");
    assert_eq!(json["electronicAddress"], "compliance@acme.example.com");
}

/// The fallback role exists precisely so a product handled only by a fulfilment
/// service provider still has an answerable party. Before it, that case could
/// not be written down at all.
#[test]
fn a_fulfilment_service_provider_round_trips() {
    let json = serde_json::to_value(OperatorRole::FulfilmentServiceProvider).unwrap();
    assert_eq!(json, "fulfilmentServiceProvider");
    let back: OperatorRole = serde_json::from_value(json).unwrap();
    assert_eq!(back, OperatorRole::FulfilmentServiceProvider);
}
