//! A second-life claim is only as good as the evidence that substantiates it,
//! and the result says which evidence that was.

use alloc::vec::Vec;

use super::{
    DerivationEdge, DocumentEvidence, Evidence, SubstantiationBasis, SubstantiationDefect,
    TransferEvidence, WasteHandoverEvidence, check_derivation_substantiation,
};

const OPERATOR: &str = "did:web:second-life.example";
const OTHER: &str = "did:web:someone-else.example";
/// SHA-256 of the ASCII bytes `test`.
const DIGEST: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
const DIGEST_UPPER: &str = "9F86D081884C7D659A2FEAA0C55AD015A3BF4F1B2B0B822CD15D6C15B0F00A08";
const NOT_HEX: &str = "gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg";
const DELIVERY_NOTE: &str = "https://example.org/evidence/delivery-note";

fn good_transfer() -> TransferEvidence<'static> {
    TransferEvidence {
        reason: "repurposing",
        incoming_operator_did: OPERATOR,
        outgoing_authorised: true,
        accepted: true,
    }
}

fn good_waste_handover() -> WasteHandoverEvidence {
    WasteHandoverEvidence {
        outgoing_authorised: true,
        accepted: true,
    }
}

fn good_document() -> DocumentEvidence<'static> {
    DocumentEvidence {
        document_type: DELIVERY_NOTE,
        sha256: DIGEST,
    }
}

/// Judge one edge claiming `operation` on `evidence`, and assert its outcome and
/// the item that outcome rests on.
#[track_caller]
fn assert_edge<'a>(
    operation: &'a str,
    evidence: &'a [Evidence<'a>],
    expected: Result<SubstantiationBasis, SubstantiationDefect<'a>>,
    expected_index: Option<usize>,
) {
    let edges = [DerivationEdge {
        operation,
        evidence,
    }];
    let outcomes = check_derivation_substantiation(OPERATOR, &edges);
    assert_eq!(outcomes.len(), 1, "one outcome per edge");
    assert_eq!(outcomes[0].edge_index, 0);
    assert_eq!(outcomes[0].outcome, expected);
    assert_eq!(outcomes[0].evidence_index, expected_index);
}

// ── A transfer of responsibility ─────────────────────────────────────────────

#[test]
fn a_transfer_the_predecessor_authorised_backs_the_edge() {
    assert_edge(
        "repurposing",
        &[Evidence::Transfer(good_transfer())],
        Ok(SubstantiationBasis::PredecessorTransfer),
        Some(0),
    );
}

#[test]
fn a_passport_with_no_derivation_edges_has_nothing_to_report() {
    assert!(check_derivation_substantiation(OPERATOR, &[]).is_empty());
}

/// The case the rule exists for: a second-life claim asserted unilaterally.
#[test]
fn an_edge_with_no_evidence_is_unsubstantiated() {
    assert_edge(
        "repurposing",
        &[],
        Err(SubstantiationDefect::NoEvidence),
        None,
    );
}

/// The predecessor's own authorisation is the half that corroborates the claim.
#[test]
fn an_unauthorised_transfer_is_not_corroboration() {
    assert_edge(
        "repurposing",
        &[Evidence::Transfer(TransferEvidence {
            outgoing_authorised: false,
            ..good_transfer()
        })],
        Err(SubstantiationDefect::OutgoingAuthorisationMissing),
        Some(0),
    );
}

#[test]
fn an_abandoned_handover_does_not_support_a_live_claim() {
    assert_edge(
        "repurposing",
        &[Evidence::Transfer(TransferEvidence {
            accepted: false,
            ..good_transfer()
        })],
        Err(SubstantiationDefect::NotAccepted),
        Some(0),
    );
}

/// A transfer to someone else is not evidence that *this* operator took over.
#[test]
fn a_transfer_to_a_third_party_is_not_evidence_for_this_passport() {
    assert_edge(
        "repurposing",
        &[Evidence::Transfer(TransferEvidence {
            incoming_operator_did: OTHER,
            ..good_transfer()
        })],
        Err(SubstantiationDefect::IncomingOperatorMismatch {
            expected: OPERATOR,
            found: OTHER,
        }),
        Some(0),
    );
}

/// Art. 77(7)'s four operations have different legal consequences, so an edge
/// and its transfer must name the same one.
#[test]
fn the_operation_and_the_transfer_reason_must_agree() {
    assert_edge(
        "remanufacturing",
        &[Evidence::Transfer(good_transfer())],
        Err(SubstantiationDefect::OperationMismatch {
            edge_operation: "remanufacturing",
            transfer_reason: "repurposing",
        }),
        Some(0),
    );
}

/// One defect per edge, and it names the most fundamental one.
///
/// Reporting a reason mismatch on a transfer nobody authorised would bury the
/// real problem under a detail.
#[test]
fn the_most_fundamental_defect_of_an_item_wins() {
    assert_edge(
        "remanufacturing",
        &[Evidence::Transfer(TransferEvidence {
            reason: "repurposing",
            incoming_operator_did: OTHER,
            outgoing_authorised: false,
            accepted: false,
        })],
        Err(SubstantiationDefect::OutgoingAuthorisationMissing),
        Some(0),
    );
}

// ── A waste handover ─────────────────────────────────────────────────────────

/// The route the original operator does take part in: its battery became waste
/// and left its hands, and a waste battery is what preparation for repurposing
/// starts from (Art. 3(30)).
#[test]
fn a_waste_handover_backs_preparation_for_repurposing() {
    assert_edge(
        "preparationForRepurposing",
        &[Evidence::WasteHandover(good_waste_handover())],
        Ok(SubstantiationBasis::PredecessorWasteHandover),
        Some(0),
    );
}

/// Art. 3(31) defines repurposing over "a battery, that is not a waste battery".
#[test]
fn a_waste_handover_contradicts_a_repurposing_edge() {
    assert_edge(
        "repurposing",
        &[Evidence::WasteHandover(good_waste_handover())],
        Err(SubstantiationDefect::WasteHandoverContradictsOperation {
            edge_operation: "repurposing",
        }),
        Some(0),
    );
}

/// Only repurposing carries the contradiction. Remanufacturing works on "a used
/// battery" (Art. 3(32)), and Art. 3(29) defines preparation for re-use through
/// Directive 2008/98/EC, which this rule does not read: neither is judged here.
#[test]
fn a_waste_handover_is_not_judged_against_the_other_operations() {
    for operation in ["preparationForReuse", "remanufacturing"] {
        assert_edge(
            operation,
            &[Evidence::WasteHandover(good_waste_handover())],
            Ok(SubstantiationBasis::PredecessorWasteHandover),
            Some(0),
        );
    }
}

/// A waste handover is a transfer record of its own reason, and its
/// corroboration rests on the same signature.
#[test]
fn a_waste_handover_needs_the_predecessors_authorisation_and_completion() {
    assert_edge(
        "preparationForRepurposing",
        &[Evidence::WasteHandover(WasteHandoverEvidence {
            outgoing_authorised: false,
            accepted: true,
        })],
        Err(SubstantiationDefect::OutgoingAuthorisationMissing),
        Some(0),
    );
    assert_edge(
        "preparationForRepurposing",
        &[Evidence::WasteHandover(WasteHandoverEvidence {
            outgoing_authorised: true,
            accepted: false,
        })],
        Err(SubstantiationDefect::NotAccepted),
        Some(0),
    );
}

// ── A document ───────────────────────────────────────────────────────────────

/// The case neither record covers: a battery that is not waste, bought through
/// several owners. Reported as a supplied document, never as corroboration.
#[test]
fn a_document_backs_the_edge_as_a_supplied_document() {
    assert_edge(
        "repurposing",
        &[Evidence::Document(good_document())],
        Ok(SubstantiationBasis::SuppliedDocument),
        Some(0),
    );
}

#[test]
fn a_document_must_say_what_it_is() {
    for document_type in ["", "   "] {
        assert_edge(
            "repurposing",
            &[Evidence::Document(DocumentEvidence {
                document_type,
                ..good_document()
            })],
            Err(SubstantiationDefect::DocumentTypeMissing),
            Some(0),
        );
    }
}

#[test]
fn a_document_digest_must_be_a_sha256() {
    for sha256 in ["", &DIGEST[..63], NOT_HEX] {
        assert_edge(
            "repurposing",
            &[Evidence::Document(DocumentEvidence {
                sha256,
                ..good_document()
            })],
            Err(SubstantiationDefect::DocumentDigestMalformed),
            Some(0),
        );
    }
    // Hexadecimal is case-insensitive.
    assert_edge(
        "repurposing",
        &[Evidence::Document(DocumentEvidence {
            sha256: DIGEST_UPPER,
            ..good_document()
        })],
        Ok(SubstantiationBasis::SuppliedDocument),
        Some(0),
    );
}

// ── Which item decides ───────────────────────────────────────────────────────

/// A document beside a signed transfer does not lower what backs the edge.
#[test]
fn the_strongest_backing_item_is_reported() {
    assert_edge(
        "repurposing",
        &[
            Evidence::Document(good_document()),
            Evidence::Transfer(good_transfer()),
        ],
        Ok(SubstantiationBasis::PredecessorTransfer),
        Some(1),
    );
}

/// An edge backed by something is backed, whatever else failed beside it. The
/// basis tells the caller how little that may be.
#[test]
fn a_backing_item_outranks_a_refused_one() {
    assert_edge(
        "repurposing",
        &[
            Evidence::Transfer(TransferEvidence {
                outgoing_authorised: false,
                ..good_transfer()
            }),
            Evidence::Document(good_document()),
        ],
        Ok(SubstantiationBasis::SuppliedDocument),
        Some(1),
    );
}

/// When nothing backs the edge, the defect reported is that of the strongest
/// kind offered, so a missing predecessor authorisation is not buried under a
/// malformed document.
#[test]
fn when_nothing_backs_the_strongest_kinds_defect_is_reported() {
    assert_edge(
        "repurposing",
        &[
            Evidence::Document(DocumentEvidence {
                sha256: "",
                ..good_document()
            }),
            Evidence::Transfer(TransferEvidence {
                outgoing_authorised: false,
                ..good_transfer()
            }),
        ],
        Err(SubstantiationDefect::OutgoingAuthorisationMissing),
        Some(1),
    );
}

/// Between two refused items of one kind, the more fundamental defect is
/// reported, wherever it sits in the list: a missing predecessor authorisation
/// is not hidden behind another transfer's operation mismatch.
#[test]
fn between_refused_items_of_one_kind_the_more_fundamental_defect_wins() {
    let unauthorised = Evidence::Transfer(TransferEvidence {
        outgoing_authorised: false,
        ..good_transfer()
    });
    let wrong_operation = Evidence::Transfer(TransferEvidence {
        reason: "remanufacturing",
        ..good_transfer()
    });

    assert_edge(
        "repurposing",
        &[wrong_operation, unauthorised],
        Err(SubstantiationDefect::OutgoingAuthorisationMissing),
        Some(1),
    );
    assert_edge(
        "repurposing",
        &[unauthorised, wrong_operation],
        Err(SubstantiationDefect::OutgoingAuthorisationMissing),
        Some(0),
    );
}

/// The same holds for documents: a missing type outranks a malformed digest.
#[test]
fn between_refused_documents_the_missing_type_wins() {
    let untyped = Evidence::Document(DocumentEvidence {
        document_type: "",
        ..good_document()
    });
    let bad_digest = Evidence::Document(DocumentEvidence {
        sha256: NOT_HEX,
        ..good_document()
    });

    assert_edge(
        "repurposing",
        &[bad_digest, untyped],
        Err(SubstantiationDefect::DocumentTypeMissing),
        Some(1),
    );
}

#[test]
fn among_equals_the_first_item_decides() {
    assert_edge(
        "repurposing",
        &[
            Evidence::Document(good_document()),
            Evidence::Document(good_document()),
        ],
        Ok(SubstantiationBasis::SuppliedDocument),
        Some(0),
    );
}

/// The plural case Art. 77(7) requires: several predecessors, each judged on its
/// own evidence, one outcome per edge in edge order.
#[test]
fn each_predecessor_is_judged_on_its_own_evidence() {
    let backed = [Evidence::Transfer(good_transfer())];
    let abandoned = [Evidence::Transfer(TransferEvidence {
        accepted: false,
        ..good_transfer()
    })];
    let waste = [Evidence::WasteHandover(good_waste_handover())];
    let edges = [
        DerivationEdge {
            operation: "repurposing",
            evidence: &backed,
        },
        DerivationEdge {
            operation: "preparationForRepurposing",
            evidence: &[],
        },
        DerivationEdge {
            operation: "repurposing",
            evidence: &abandoned,
        },
        DerivationEdge {
            operation: "preparationForRepurposing",
            evidence: &waste,
        },
    ];

    let summary: Vec<_> = check_derivation_substantiation(OPERATOR, &edges)
        .iter()
        .map(|outcome| (outcome.edge_index, outcome.outcome))
        .collect();
    assert_eq!(
        summary,
        [
            (0, Ok(SubstantiationBasis::PredecessorTransfer)),
            (1, Err(SubstantiationDefect::NoEvidence)),
            (2, Err(SubstantiationDefect::NotAccepted)),
            (3, Ok(SubstantiationBasis::PredecessorWasteHandover)),
        ]
    );
}
