//! Does a passport's life status agree with the edges that produced it?
//!
//! # Why the status is stored and then checked, rather than derived
//!
//! Deriving the status from `derived_from` is tempting and does not survive the
//! plural case. Art. 77(7) permits several predecessors and nothing forces them
//! to share an operation, so a unit built from one repurposed and one
//! remanufactured predecessor has no unambiguous derived status. Storing it and
//! checking it recovers the one real advantage of deriving — a status that
//! cannot silently contradict the lineage — without inheriting that flaw. Same
//! shape as the derivation-consent rule next door.
//!
//! # What "agrees" means, and why it is *some* rather than *every*
//!
//! A claimed second-life status must be supported by **at least one** edge. It
//! deliberately does not require every edge to agree: that would make the plural
//! mixed-predecessor case above unrepresentable, and that case is lawful. What it
//! catches is the claim no edge supports at all — a passport claiming
//! `remanufactured` whose only derivation edge says `repurposing` is inconsistent
//! on its face.
//!
//! # `waste` is exempt, and that is not an oversight
//!
//! The four second-life values describe how a unit was *made*, and each of the
//! four operations produces a new passport under Art. 77(7), so a second-life
//! unit is born knowing its status. `waste` is the one value that is a transition
//! **on a record that continues**: Art. 77(7)'s second subparagraph moves
//! responsibility on a battery becoming waste and mandates no new passport.
//!
//! So a waste battery's derivation edges still describe its manufacture, and they
//! say nothing about whether it is now waste. A repurposed unit that later became
//! waste carries a `repurposing` edge and a `waste` status, and both are correct.
//! Checking `waste` against the edges would report that entirely ordinary record
//! as inconsistent.
//!
//! # What this rule cannot do
//!
//! It cannot tell whether an edge's operation is *truthful* — only whether the
//! status and the edges tell the same story. Whether the predecessor's operator
//! consented to the edge at all is [`super::consent`]'s question, and the two are
//! independent: an edge can be perfectly consented and still support a status the
//! passport does not claim.

use super::status_defect::StatusDefect;

/// Operations that produce a unit with the given life status, in wire form.
///
/// `repurposed` has two, and the reason is the input rather than the outcome:
/// Art. 3(30) (`preparationForRepurposing`) operates on "a waste battery, or
/// parts thereof", Art. 3(31) (`repurposing`) on "a battery, that is not a waste
/// battery". Both yield a repurposed unit.
///
/// `None` means the status is not one Annex XIII point 4(c) enumerates.
fn supporting_operations(life_status: &str) -> Option<&'static [&'static str]> {
    match life_status {
        "original" => Some(&[]),
        "repurposed" => Some(&["repurposing", "preparationForRepurposing"]),
        "re-used" => Some(&["preparationForReuse"]),
        "remanufactured" => Some(&["remanufacturing"]),
        _ => None,
    }
}

/// Check a passport's life status against the operations its derivation edges
/// claim.
///
/// `life_status` and `operations` are wire forms — `"re-used"`,
/// `"preparationForReuse"` — because the two vocabularies are defined in a crate
/// this one must not depend on. They are pinned byte-identical on the core side.
///
/// Returns `None` when the record is consistent, including the two cases that
/// are consistent by construction: a passport with no life status at all (the
/// vocabulary is Reg. (EU) 2023/1542's, and a product group whose instrument does
/// not ask the question should not answer it), and a `waste` status, for the
/// reason in this module's documentation.
#[must_use]
pub fn check_life_status_consistency<'a>(
    life_status: Option<&'a str>,
    operations: &[&'a str],
) -> Option<StatusDefect<'a>> {
    let status = life_status?;

    // A transition on a record that continues, not a claim about manufacture.
    if status == "waste" {
        return None;
    }

    let Some(supporting) = supporting_operations(status) else {
        return Some(StatusDefect::UnknownStatus { status });
    };

    if status == "original" {
        // An original unit is the one placed on the market, not the output of an
        // operation on something else. Any derivation edge contradicts it.
        return operations
            .first()
            .copied()
            .map(|operation| StatusDefect::OriginalIsDerived { operation });
    }

    if operations
        .iter()
        .any(|operation| supporting.contains(operation))
    {
        return None;
    }

    Some(StatusDefect::NoEdgeSupportsStatus { status })
}
