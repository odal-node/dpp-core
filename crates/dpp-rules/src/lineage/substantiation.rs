//! Is a second-life claim substantiated by a transfer of responsibility?
//!
//! # The gap this closes
//!
//! A derivation edge pins its target with a hash. That proves the target has not
//! been *modified*; it says nothing about whether the operation the edge names
//! ever took place. Anyone can publish a passport claiming to derive from anyone
//! else's product.
//!
//! For a bill of materials that is tolerable — over-claiming a supplier is a
//! commercial problem, and a component reference is openly a claim by the
//! assembler. For second life it is not. A derivation claim asserts that a unit
//! left its original operator's hands and was placed on the market again.
//! Believed, that purports to end the original operator's Art. 77(4) duty to keep
//! the original passport accurate, complete and up to date, for a unit that may
//! still be on the market under their name.
//!
//! # What Art. 77(7) says, and what it does not
//!
//! Reg. (EU) 2023/1542 **Art. 77(7)**: "For a battery that has been subject to
//! preparation for re-use, preparation for repurposing, repurposing or
//! remanufacturing, the responsibility for the fulfilment of the obligations under
//! paragraph 4 of this Article shall be transferred to the economic operator that
//! has placed that battery on the market or has put it into service. Such battery
//! shall have a new battery passport linked to the battery passport or passports
//! of the original battery or batteries."
//!
//! The responsibility moves **by operation of law**, to whoever placed the
//! second-life unit on the market. The predecessor's agreement is not a
//! precondition and the provision does not mention it. An operator asserting a
//! derivation is not assigning responsibility to somebody else; it is assuming it
//! itself. The same shape appears in the general regime, ESPR Art. 11(d): where a
//! new passport is created for a product that already has one, it "shall be
//! linked to the original digital product passport or passports".
//!
//! The only *authorisation* in Art. 77 is paragraph 4's: an operator "may give
//! written authorisation to any other operator to act on its behalf". That
//! delegates an operator's own duty and is a different thing from the transfer in
//! paragraph 7.
//!
//! So this rule does not ask whether the predecessor *consented*. It asks whether
//! the claim is *substantiated*: whether evidence that the operation took place
//! accompanies it.
//!
//! # What counts as the evidence, and what it does not prove
//!
//! A transfer-of-responsibility record is that evidence. A second-life operation
//! *is* a transfer under Art. 77(7), so a record of one can corroborate the other,
//! and a record that names a different operation, or a different incoming
//! operator, is no evidence of this claim. But the record is an implementation
//! artefact. Art. 77(7)'s transfer is a legal consequence that follows the
//! operation; nothing in the article requires a bilateral instrument to exist or
//! to be recorded, and the absence of one is **not a breach** of it. This rule
//! asks for the record because it is the one corroborating artefact the model
//! has, not because the Regulation does.
//!
//! # What the corroboration actually rests on
//!
//! Precisely one signature in a transfer record is a counterparty's own: the
//! **outgoing** operator's authorisation, produced with that operator's key. The
//! acceptance half is the hosting node's attestation that the acceptance step
//! ran — it is not a signature by the incoming operator, who has no key on the
//! node recording it.
//!
//! That asymmetry happens to be harmless here, and the reason is worth stating
//! because it is easy to get backwards. The corroboration this rule asks for is the
//! **predecessor's**: the operator being derived *from* is the party best placed
//! to confirm that the unit left its hands. In a second-life transfer the
//! predecessor's operator is the outgoing party, so the half that is a genuine
//! party signature is exactly the half that matters. The incoming half is this
//! passport's own operator asserting something about itself, which was never
//! evidence of anything.
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
//! [`DerivationEdge::transfer`] as `None` and is reported as unsubstantiated —
//! which is the correct reading of "no evidence was found", not an assumption that
//! none exists.

use alloc::vec::Vec;

use super::finding::{SubstantiationDefect, SubstantiationFinding};
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
/// A passport with no derivation edges is trivially substantiated — this says
/// nothing about bill-of-materials edges, which deliberately carry no
/// substantiation requirement at all.
#[must_use]
pub fn check_derivation_substantiation<'a>(
    passport_operator_did: &'a str,
    edges: &'a [DerivationEdge<'a>],
) -> Vec<SubstantiationFinding<'a>> {
    let mut findings = Vec::new();

    for (edge_index, edge) in edges.iter().enumerate() {
        if let Some(defect) = edge_defect(passport_operator_did, edge) {
            findings.push(SubstantiationFinding { edge_index, defect });
        }
    }

    findings
}

/// The single most fundamental defect on one edge, if any.
fn edge_defect<'a>(
    passport_operator_did: &'a str,
    edge: &DerivationEdge<'a>,
) -> Option<SubstantiationDefect<'a>> {
    let Some(transfer) = edge.transfer else {
        return Some(SubstantiationDefect::NoTransfer);
    };

    if !transfer.outgoing_authorised {
        return Some(SubstantiationDefect::OutgoingAuthorisationMissing);
    }
    if !transfer.accepted {
        return Some(SubstantiationDefect::NotAccepted);
    }
    if transfer.incoming_operator_did != passport_operator_did {
        return Some(SubstantiationDefect::IncomingOperatorMismatch {
            expected: passport_operator_did,
            found: transfer.incoming_operator_did,
        });
    }
    if transfer.reason != edge.operation {
        return Some(SubstantiationDefect::OperationMismatch {
            edge_operation: edge.operation,
            transfer_reason: transfer.reason,
        });
    }

    None
}
