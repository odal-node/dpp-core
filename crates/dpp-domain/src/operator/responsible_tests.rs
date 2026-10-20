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
        identifier: None,
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

fn operator_with(
    id: Option<&str>,
    scheme: Option<&str>,
    typed: Option<crate::identifier::ActorIdentifier>,
) -> ResponsibleOperator {
    ResponsibleOperator {
        did: "did:web:acme.example.com".into(),
        name: "Acme GmbH".into(),
        role: OperatorRole::Manufacturer,
        eu_operator_id: id.map(Into::into),
        eu_operator_id_scheme: scheme.map(Into::into),
        identifier: typed,
        country: "DE".into(),
        registered_trade_name: None,
        postal_address: None,
        electronic_address: None,
    }
}

/// 🚨 The point (k) identifier is typed additively. An operator stored before the
/// field existed reads, and is written back without the key, so a signed transfer
/// chain keeps its bytes.
#[test]
fn an_operator_without_a_typed_identifier_reads_and_is_written_back_unchanged() {
    let stored = serde_json::json!({
        "did": "did:web:acme.example.com",
        "name": "Acme GmbH",
        "role": "manufacturer",
        "euOperatorId": "DE123456789",
        "euOperatorIdScheme": "vat",
        "country": "DE"
    });
    let read: ResponsibleOperator = serde_json::from_value(stored.clone()).expect("an old record");
    assert_eq!(read.identifier, None);
    assert_eq!(serde_json::to_value(&read).unwrap(), stored);
}

/// The point (k) identifier as an EN 18219 clause 6 identifier: the typed field
/// when there is one, the legacy pair at an operator otherwise, and `None` when the
/// operator states nothing.
#[test]
fn the_point_k_identifier_is_read_as_a_clause_6_identifier() {
    use crate::identifier::{ActorIdentifier, Gln, GlnAi, Icd};

    const LEI: &str = "529900T8BM49AURSDO55";
    const GLN: &str = "9506000134352";

    // Nothing stated.
    assert_eq!(operator_with(None, None, None).actor_identifier(), None);

    // The legacy pair, placed.
    assert_eq!(
        operator_with(Some(LEI), Some("lei"), None).actor_identifier(),
        Some(ActorIdentifier::iso6523(Icd::lei(), LEI))
    );
    // A `gln` is a party at an operator.
    assert_eq!(
        operator_with(Some(GLN), Some("gln"), None).actor_identifier(),
        Some(Ok(ActorIdentifier::gs1(
            GlnAi::Party,
            Gln::parse(GLN).expect("a GLN")
        )))
    );

    // The typed field is what the operator states, whatever the legacy pair says.
    let typed = ActorIdentifier::iso6523(Icd::lei(), LEI).expect("an LEI");
    assert_eq!(
        operator_with(Some("DE123456789"), Some("vat"), Some(typed.clone())).actor_identifier(),
        Some(Ok(typed))
    );
}

/// 🚨 What has no clause 6 route reads, and says it is unplaced. A value with no
/// scheme is the same: it does not say what it is, and guessing would be a false
/// claim and not a missing one.
#[test]
fn a_point_k_identifier_with_no_clause_6_route_still_reads_and_is_unplaced() {
    use crate::identifier::ActorIdentifierError;

    assert_eq!(
        operator_with(Some("DE123456789"), Some("vat"), None).actor_identifier(),
        Some(Err(ActorIdentifierError::Unmapped {
            scheme: "vat".into()
        }))
    );
    assert_eq!(
        operator_with(Some("DE123456789012345"), Some("eori"), None).actor_identifier(),
        Some(Err(ActorIdentifierError::Unmapped {
            scheme: "eori".into()
        }))
    );
    assert_eq!(
        operator_with(Some("DE123456789"), None, None).actor_identifier(),
        Some(Err(ActorIdentifierError::Unmapped { scheme: "".into() }))
    );
}

/// 🚨 An operator whose typed identifier is in a scheme this build does not know
/// still reads, and is written back unchanged. The record is in the signed public
/// view and the transfer chain, so it can come from a build that knows the
/// scheme, and refusing the identifier would refuse the passport.
#[test]
fn an_operator_with_an_identifier_in_a_later_scheme_reads_and_is_written_back() {
    use crate::identifier::ActorIdentifier;

    let stored = serde_json::json!({
        "did": "did:web:acme.example.com",
        "name": "Acme GmbH",
        "role": "manufacturer",
        "euOperatorId": "10.1000/182",
        "identifier": { "scheme": "doi", "value": "10.1000/182" },
        "country": "DE"
    });
    let read: ResponsibleOperator =
        serde_json::from_value(stored.clone()).expect("a record from a later build reads");
    assert!(matches!(
        read.actor_identifier(),
        Some(Ok(ActorIdentifier::Unrecognised(_)))
    ));
    assert_eq!(serde_json::to_value(&read).unwrap(), stored);
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
