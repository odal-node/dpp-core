//! Does a second-life claim carry the predecessor operator's authorisation?
//!
//! # The gap this closes
//!
//! A derivation edge pins its target with a hash. That proves the target has not
//! been *modified*; it says nothing about whether the target's operator agreed to
//! the relationship. Anyone can publish a passport claiming to derive from anyone
//! else's product.
//!
//! For a bill of materials that is tolerable — over-claiming a supplier is a
//! commercial problem, and a component reference is openly a claim by the
//! assembler. For second life it is not. Reg. (EU) 2023/1542 **Art. 77(7)** moves
//! regulatory responsibility to the operator placing the second-life unit on the
//! market, and responsibility must not be assignable by unilateral assertion.
//!
//! The fix is not a third mechanism. A transfer-of-responsibility record already
//! carries the outgoing operator's own authorisation, and a second-life operation
//! *is* a transfer under Art. 77(7) — so the two describe one event and should be
//! required to agree.
//!
//! # What the authorisation actually rests on
//!
//! Precisely one signature in a transfer record is a counterparty's own: the
//! **outgoing** operator's authorisation, produced with that operator's key. The
//! acceptance half is the hosting node's attestation that the acceptance step
//! ran — it is not a signature by the incoming operator, who has no key on the
//! node recording it.
//!
//! That asymmetry happens to be harmless here, and the reason is worth stating
//! because it is easy to get backwards. The consent this rule needs is the
//! **predecessor's**: the question is whether the operator being derived *from*
//! agreed. In a second-life transfer the predecessor's operator is the outgoing
//! party, so the half that is a genuine party signature is exactly the half that
//! matters. The incoming half is this passport's own operator asserting something
//! about itself, which was never evidence of anything.
//!
//! An earlier statement of this design justified the rule by saying a transfer
//! record is "dual-signed by both operators". It is not, and this rule does not
//! rely on that. [`TransferEvidence::accepted`](super::input::TransferEvidence::accepted)
//! is carried and checked because an abandoned or rejected handover should not
//! support a live claim, not because it is a second party's signature.
//!
//! # What this rule cannot do
//!
//! It cannot tell whether a transfer record actually concerns the predecessor a
//! given edge points at. An edge identifies its target by URI; a transfer record
//! identifies its subject by passport id, and resolving one to the other is a
//! network fetch. **The caller correlates; this rule checks.** An edge whose
//! caller found no corresponding transfer arrives here with
//! [`DerivationEdge::transfer`] as `None` and is reported as unconsented — which
//! is the correct reading of "no evidence was found", not an assumption that none
//! exists.

use alloc::vec::Vec;

use super::finding::{ConsentDefect, ConsentFinding};
use super::input::DerivationEdge;

/// Check every derivation edge against the transfer correlated to it.
///
/// Returns one finding per unsupported edge, in edge order. An empty result
/// means every edge is backed by a completed transfer, authorised by the
/// outgoing operator, naming the same operation, and handing responsibility to
/// this passport's operator.
///
/// At most one finding per edge: the defects are ordered by how fundamental they
/// are, and reporting "the operation disagrees" about a transfer that was never
/// authorised would bury the real problem. A caller that fixes the reported
/// defect and re-runs sees the next one.
///
/// A passport with no derivation edges is trivially consented — this says
/// nothing about bill-of-materials edges, which deliberately carry no consent
/// requirement at all.
#[must_use]
pub fn check_derivation_consent<'a>(
    passport_operator_did: &'a str,
    edges: &'a [DerivationEdge<'a>],
) -> Vec<ConsentFinding<'a>> {
    let mut findings = Vec::new();

    for (edge_index, edge) in edges.iter().enumerate() {
        if let Some(defect) = edge_defect(passport_operator_did, edge) {
            findings.push(ConsentFinding { edge_index, defect });
        }
    }

    findings
}

/// The single most fundamental defect on one edge, if any.
fn edge_defect<'a>(
    passport_operator_did: &'a str,
    edge: &DerivationEdge<'a>,
) -> Option<ConsentDefect<'a>> {
    let Some(transfer) = edge.transfer else {
        return Some(ConsentDefect::NoTransfer);
    };

    if !transfer.outgoing_authorised {
        return Some(ConsentDefect::OutgoingAuthorisationMissing);
    }
    if !transfer.accepted {
        return Some(ConsentDefect::NotAccepted);
    }
    if transfer.incoming_operator_did != passport_operator_did {
        return Some(ConsentDefect::IncomingOperatorMismatch {
            expected: passport_operator_did,
            found: transfer.incoming_operator_did,
        });
    }
    if transfer.reason != edge.operation {
        return Some(ConsentDefect::OperationMismatch {
            edge_operation: edge.operation,
            transfer_reason: transfer.reason,
        });
    }

    None
}
