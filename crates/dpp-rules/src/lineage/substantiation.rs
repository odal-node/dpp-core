//! Is a second-life claim substantiated by evidence that the operation happened?
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
//! # What counts as evidence, and what each kind proves
//!
//! Art. 77(7) names no evidence. Nothing in it requires a bilateral instrument to
//! exist or to be recorded, so a claim with none is **unsubstantiated, not a
//! breach**. The rule accepts three kinds, and reports which one backed each edge
//! ([`SubstantiationBasis`]), because they do not prove the same thing:
//!
//! - **A transfer of responsibility** on the predecessor, to this passport's
//!   operator, for the operation the edge claims. A second-life operation *is* a
//!   transfer under Art. 77(7), so a record of one corroborates the other, and the
//!   predecessor's operator signs it. The strongest kind.
//! - **A waste handover** on the predecessor, under Art. 77(7) second
//!   subparagraph: the battery became waste and responsibility left its operator
//!   for a producer, a producer responsibility organisation or a waste management
//!   operator. The predecessor's operator signs this one too. A battery can reach
//!   a second-life operator through the waste stream without its original
//!   operator ever handing it over, and this is the record that original operator
//!   does produce. It shows the battery left the predecessor's hands, not that it
//!   reached this one.
//! - **A document** from the value chain — a delivery note, a purchase record —
//!   supplied by this passport's operator and pinned by its digest. It covers what
//!   the other two cannot, such as a battery that is not waste bought through
//!   several owners. Nobody but the claiming operator stands behind it.
//!
//! The document's type is an identifier the caller chooses, never a list fixed
//! here: see [`DocumentEvidence::document_type`] for why.
//!
//! One consistency is checked across kinds. Art. 3(31) defines repurposing over
//! "a battery, that is not a waste battery", so a waste handover contradicts a
//! `repurposing` edge; the operation on a waste battery is preparation for
//! repurposing, Art. 3(30). Art. 3(29) defines preparation for re-use by reference
//! to Directive 2008/98/EC, and nothing here says whether that operation starts
//! from waste.
//!
//! # What the result leaves to the caller
//!
//! Which bases to accept is the caller's policy, and the rule does not set it.
//! Counting a document the claiming operator supplied the same as the
//! predecessor's own signature would reopen the gap this rule closes, so the
//! basis is reported rather than folded into a yes or no.
//!
//! # What the corroboration actually rests on
//!
//! Precisely one signature in a transfer record is a counterparty's own: the
//! **outgoing** operator's authorisation, produced with that operator's key. The
//! acceptance half is the hosting node's attestation that the acceptance step
//! ran — it is not a signature by the incoming operator, who has no key on the
//! node recording it. The same holds for a waste handover, which is a transfer
//! record of its own reason.
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
//! rely on that. [`TransferEvidence::accepted`] is carried and checked because an abandoned or rejected handover should not
//! support a live claim, not because it is a second party's signature.
//!
//! # What this rule cannot do
//!
//! It cannot tell whether an item of evidence actually concerns the predecessor a
//! given edge points at. An edge identifies its target by URI; a transfer record
//! identifies its subject by passport id, and resolving one to the other is a
//! network fetch. Nor can it read a document, so whether one is genuine and
//! matches its digest is the caller's to check. **The caller correlates and
//! verifies; this rule judges.** An edge whose caller found nothing arrives here
//! with empty [`DerivationEdge::evidence`] and is reported as unsubstantiated —
//! which is the correct reading of "no evidence was found", not an assumption that
//! none exists.

use alloc::vec::Vec;

use super::basis::SubstantiationBasis;
use super::evidence::{DocumentEvidence, TransferEvidence, WasteHandoverEvidence};
use super::finding::{EdgeSubstantiation, SubstantiationDefect};
use super::input::{DerivationEdge, Evidence};

/// The wire form of the one operation a waste handover contradicts: Art. 3(31)
/// repurposing, which is defined over a battery that is not waste.
const REPURPOSING: &str = "repurposing";

/// Judge every derivation edge on the evidence the caller found for it.
///
/// Returns one outcome per edge, in edge order. An edge is backed by the
/// strongest item that backs it, in [`SubstantiationBasis`] order, the first in
/// list order among equals. An edge that nothing backs reports one defect: the
/// most fundamental defect among the items of its strongest kind, the first in
/// list order among equals, so a missing predecessor authorisation is buried
/// neither under a detail about a document nor under a lesser defect of a
/// second transfer. An edge with no evidence reports
/// [`SubstantiationDefect::NoEvidence`].
///
/// A passport with no derivation edges gets no outcomes — this says nothing
/// about bill-of-materials edges, which deliberately carry no substantiation
/// requirement at all.
#[must_use]
pub fn check_derivation_substantiation<'a>(
    passport_operator_did: &'a str,
    edges: &'a [DerivationEdge<'a>],
) -> Vec<EdgeSubstantiation<'a>> {
    edges
        .iter()
        .enumerate()
        .map(|(edge_index, edge)| {
            let (evidence_index, outcome) = match decisive(passport_operator_did, edge) {
                Some((index, outcome)) => (Some(index), outcome),
                None => (None, Err(SubstantiationDefect::NoEvidence)),
            };
            EdgeSubstantiation {
                edge_index,
                outcome,
                evidence_index,
            }
        })
        .collect()
}

/// The item an edge's outcome rests on, and that outcome: a backing item over
/// a refused one, then the stronger kind, then (between refused items) the more
/// fundamental defect, then the earlier item. `None` for an edge with no
/// evidence.
fn decisive<'a>(
    passport_operator_did: &'a str,
    edge: &DerivationEdge<'a>,
) -> Option<(usize, Result<SubstantiationBasis, SubstantiationDefect<'a>>)> {
    edge.evidence
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let outcome = judge(passport_operator_did, edge.operation, item);
            (index, strength(item), outcome)
        })
        .min_by_key(|(index, strength, outcome)| {
            let defect = outcome.as_ref().err().map_or(0, fundamentality);
            (outcome.is_err(), *strength, defect, *index)
        })
        .map(|(index, _, outcome)| (index, outcome))
}

/// Rank of a defect, most fundamental first, in the order the `judge_*`
/// functions check: a predecessor's authorisation before its completion, both
/// before what the record says. Only defects of one kind are ever compared,
/// since [`strength`] decides between kinds first.
fn fundamentality(defect: &SubstantiationDefect<'_>) -> u8 {
    match defect {
        SubstantiationDefect::NoEvidence
        | SubstantiationDefect::OutgoingAuthorisationMissing
        | SubstantiationDefect::DocumentTypeMissing => 0,
        SubstantiationDefect::NotAccepted | SubstantiationDefect::DocumentDigestMalformed => 1,
        SubstantiationDefect::IncomingOperatorMismatch { .. } => 2,
        SubstantiationDefect::OperationMismatch { .. }
        | SubstantiationDefect::WasteHandoverContradictsOperation { .. } => 3,
    }
}

/// Rank of an item's kind, strongest first, matching [`SubstantiationBasis`].
fn strength(item: &Evidence<'_>) -> u8 {
    match item {
        Evidence::Transfer(_) => 0,
        Evidence::WasteHandover(_) => 1,
        Evidence::Document(_) => 2,
    }
}

/// What one item backs, or its single most fundamental defect.
fn judge<'a>(
    passport_operator_did: &'a str,
    operation: &'a str,
    item: &Evidence<'a>,
) -> Result<SubstantiationBasis, SubstantiationDefect<'a>> {
    match *item {
        Evidence::Transfer(transfer) => judge_transfer(passport_operator_did, operation, transfer),
        Evidence::WasteHandover(handover) => judge_waste_handover(operation, handover),
        Evidence::Document(document) => judge_document(document),
    }
}

fn judge_transfer<'a>(
    passport_operator_did: &'a str,
    operation: &'a str,
    transfer: TransferEvidence<'a>,
) -> Result<SubstantiationBasis, SubstantiationDefect<'a>> {
    if !transfer.outgoing_authorised {
        return Err(SubstantiationDefect::OutgoingAuthorisationMissing);
    }
    if !transfer.accepted {
        return Err(SubstantiationDefect::NotAccepted);
    }
    if transfer.incoming_operator_did != passport_operator_did {
        return Err(SubstantiationDefect::IncomingOperatorMismatch {
            expected: passport_operator_did,
            found: transfer.incoming_operator_did,
        });
    }
    if transfer.reason != operation {
        return Err(SubstantiationDefect::OperationMismatch {
            edge_operation: operation,
            transfer_reason: transfer.reason,
        });
    }
    Ok(SubstantiationBasis::PredecessorTransfer)
}

fn judge_waste_handover(
    operation: &str,
    handover: WasteHandoverEvidence,
) -> Result<SubstantiationBasis, SubstantiationDefect<'_>> {
    if !handover.outgoing_authorised {
        return Err(SubstantiationDefect::OutgoingAuthorisationMissing);
    }
    if !handover.accepted {
        return Err(SubstantiationDefect::NotAccepted);
    }
    if operation == REPURPOSING {
        return Err(SubstantiationDefect::WasteHandoverContradictsOperation {
            edge_operation: operation,
        });
    }
    Ok(SubstantiationBasis::PredecessorWasteHandover)
}

fn judge_document(
    document: DocumentEvidence<'_>,
) -> Result<SubstantiationBasis, SubstantiationDefect<'_>> {
    if document.document_type.trim().is_empty() {
        return Err(SubstantiationDefect::DocumentTypeMissing);
    }
    let digest = document.sha256;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(SubstantiationDefect::DocumentDigestMalformed);
    }
    Ok(SubstantiationBasis::SuppliedDocument)
}
