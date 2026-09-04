//! A second-life claim is only as good as the transfer that consents to it.

use super::{ConsentDefect, DerivationEdge, TransferEvidence, check_derivation_consent};

const OPERATOR: &str = "did:web:second-life.example";
const OTHER: &str = "did:web:someone-else.example";

fn good_transfer() -> TransferEvidence<'static> {
    TransferEvidence {
        reason: "repurposing",
        incoming_operator_did: OPERATOR,
        outgoing_authorised: true,
        accepted: true,
    }
}

fn edge_with(transfer: TransferEvidence<'static>) -> DerivationEdge<'static> {
    DerivationEdge {
        operation: "repurposing",
        transfer: Some(transfer),
    }
}

#[test]
fn a_fully_supported_edge_produces_no_finding() {
    let edges = [edge_with(good_transfer())];
    assert!(check_derivation_consent(OPERATOR, &edges).is_empty());
}

#[test]
fn a_passport_with_no_derivation_edges_is_trivially_consented() {
    assert!(check_derivation_consent(OPERATOR, &[]).is_empty());
}

/// The case the rule exists for: a second-life claim asserted unilaterally.
#[test]
fn an_edge_with_no_transfer_is_unconsented() {
    let edges = [DerivationEdge {
        operation: "repurposing",
        transfer: None,
    }];

    let findings = check_derivation_consent(OPERATOR, &edges);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].edge_index, 0);
    assert_eq!(findings[0].defect, ConsentDefect::NoTransfer);
}

/// The predecessor's own authorisation is the half that carries the consent.
#[test]
fn an_unauthorised_transfer_is_not_consent() {
    let edges = [edge_with(TransferEvidence {
        outgoing_authorised: false,
        ..good_transfer()
    })];

    let findings = check_derivation_consent(OPERATOR, &edges);
    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].defect,
        ConsentDefect::OutgoingAuthorisationMissing
    );
}

#[test]
fn an_abandoned_handover_does_not_support_a_live_claim() {
    let edges = [edge_with(TransferEvidence {
        accepted: false,
        ..good_transfer()
    })];

    let findings = check_derivation_consent(OPERATOR, &edges);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].defect, ConsentDefect::NotAccepted);
}

/// A transfer to someone else is not evidence that *this* operator took over.
#[test]
fn a_transfer_to_a_third_party_is_not_evidence_for_this_passport() {
    let edges = [edge_with(TransferEvidence {
        incoming_operator_did: OTHER,
        ..good_transfer()
    })];

    let findings = check_derivation_consent(OPERATOR, &edges);
    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].defect,
        ConsentDefect::IncomingOperatorMismatch {
            expected: OPERATOR,
            found: OTHER,
        }
    );
}

/// Art. 77(7)'s four operations have different legal consequences, so an edge
/// and its transfer must name the same one.
#[test]
fn the_operation_and_the_transfer_reason_must_agree() {
    let edges = [DerivationEdge {
        operation: "remanufacturing",
        transfer: Some(good_transfer()),
    }];

    let findings = check_derivation_consent(OPERATOR, &edges);
    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].defect,
        ConsentDefect::OperationMismatch {
            edge_operation: "remanufacturing",
            transfer_reason: "repurposing",
        }
    );
}

/// The plural case Art. 77(7) requires: several predecessors, reported per edge.
#[test]
fn each_predecessor_is_judged_on_its_own_evidence() {
    let edges = [
        edge_with(good_transfer()),
        DerivationEdge {
            operation: "preparationForRepurposing",
            transfer: None,
        },
        edge_with(TransferEvidence {
            accepted: false,
            ..good_transfer()
        }),
    ];

    let findings = check_derivation_consent(OPERATOR, &edges);
    assert_eq!(findings.len(), 2, "the supported edge must not be reported");
    assert_eq!(findings[0].edge_index, 1);
    assert_eq!(findings[0].defect, ConsentDefect::NoTransfer);
    assert_eq!(findings[1].edge_index, 2);
    assert_eq!(findings[1].defect, ConsentDefect::NotAccepted);
}

/// One finding per edge, and it names the most fundamental defect.
///
/// Reporting a reason mismatch on a transfer nobody authorised would bury the
/// real problem under a detail.
#[test]
fn the_most_fundamental_defect_wins() {
    let edges = [DerivationEdge {
        operation: "remanufacturing",
        transfer: Some(TransferEvidence {
            reason: "repurposing",
            incoming_operator_did: OTHER,
            outgoing_authorised: false,
            accepted: false,
        }),
    }];

    let findings = check_derivation_consent(OPERATOR, &edges);
    assert_eq!(
        findings.len(),
        1,
        "one finding per edge, not one per defect"
    );
    assert_eq!(
        findings[0].defect,
        ConsentDefect::OutgoingAuthorisationMissing,
        "the missing predecessor authorisation outranks the other three"
    );
}
