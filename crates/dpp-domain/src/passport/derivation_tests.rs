//! Every `SecondLifeOperation` variant is reachable, and its vocabulary agrees
//! with `TransferReason`'s.

use super::derivation::{DerivationRef, SecondLifeOperation};
use super::reference::PassportRef;
use crate::transfer::TransferReason;

/// `ALL` must list every variant — see `TransferReason`'s equivalent test for
/// why this is two stages rather than one.
#[test]
fn all_lists_every_variant() {
    for operation in SecondLifeOperation::ALL {
        match operation {
            SecondLifeOperation::PreparationForReuse
            | SecondLifeOperation::PreparationForRepurposing
            | SecondLifeOperation::Repurposing
            | SecondLifeOperation::Remanufacturing => {}
        }
    }
    assert_eq!(
        SecondLifeOperation::ALL.len(),
        4,
        "a variant was added to the match above but not to ALL"
    );
}

/// The two vocabularies must stay byte-identical on the wire.
///
/// The rule binding a derivation edge to the transfer of responsibility that
/// consents to it matches a `SecondLifeOperation` against a `TransferReason`.
/// If the two
/// spell an operation differently, that rule silently stops matching for the
/// operation that drifted — and it fails open, since a non-match reads as "no
/// consent recorded" rather than as a bug.
#[test]
fn wire_vocabulary_agrees_with_transfer_reason() {
    let pairs = [
        (
            SecondLifeOperation::PreparationForReuse,
            TransferReason::PreparationForReuse,
        ),
        (
            SecondLifeOperation::PreparationForRepurposing,
            TransferReason::PreparationForRepurposing,
        ),
        (
            SecondLifeOperation::Repurposing,
            TransferReason::Repurposing,
        ),
        (
            SecondLifeOperation::Remanufacturing,
            TransferReason::Remanufacturing,
        ),
    ];

    assert_eq!(
        pairs.len(),
        SecondLifeOperation::ALL.len(),
        "every operation needs a TransferReason counterpart in this table"
    );

    for (operation, reason) in &pairs {
        assert_eq!(
            operation.wire_str(),
            reason.wire_str(),
            "{operation:?} and its TransferReason counterpart must agree on the wire"
        );
    }
}

/// The camelCase serde form and the hand-written `wire_str` must not diverge.
///
/// `wire_str` is spelled out rather than derived, which is what makes a rename
/// safe — and also what lets the two drift silently if nobody checks.
#[test]
fn serde_form_matches_wire_str() {
    for operation in SecondLifeOperation::ALL {
        let json = serde_json::to_string(operation).expect("operation serialises");
        assert_eq!(
            json,
            format!("\"{}\"", operation.wire_str()),
            "serde and wire_str disagree for {operation:?}"
        );
    }
}

/// A derivation edge round-trips with its operation intact.
#[test]
fn derivation_ref_round_trips() {
    let edge = DerivationRef {
        reference: PassportRef {
            uri: "https://example.test/dpp/abc".into(),
            public_jws_hash: "a".repeat(64),
        },
        operation: SecondLifeOperation::PreparationForRepurposing,
    };

    let json = serde_json::to_value(&edge).expect("edge serialises");
    assert_eq!(
        json["operation"], "preparationForRepurposing",
        "the operation must survive as its wire form"
    );
    assert_eq!(
        json["reference"]["publicJwsHash"],
        "a".repeat(64),
        "the wrapped reference keeps its own camelCase shape"
    );

    let back: DerivationRef = serde_json::from_value(json).expect("edge deserialises");
    assert_eq!(back, edge);
}
