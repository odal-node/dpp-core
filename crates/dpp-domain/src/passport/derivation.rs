//! [`DerivationRef`] — a typed upward edge to a second-life predecessor.

use serde::{Deserialize, Serialize};

use super::reference::PassportRef;

/// One of the four second-life operations named by Reg. (EU) 2023/1542
/// Art. 77(7).
///
/// ✅ COMPLIANCE-PIN: EU 2023/1542, Art. 77(7) and Art. 3(29)–(32)
/// (OJ L 191, 28.7.2023, pp. 73 and 27). Art. 77(7) transfers responsibility for
/// "a battery that has been subject to preparation for re-use, preparation for
/// repurposing, repurposing or remanufacturing"; the four are defined terms at
/// Art. 3(29) to (32).
///
/// The edge must say which one occurred because they are not interchangeable:
/// each has its own definition, and the boundary between the two repurposing
/// operations is the **waste status of the input** — Art. 3(30) operates on "a
/// waste battery, or parts thereof", Art. 3(31) on "a battery, that is not a
/// waste battery".
///
/// Mirrors the [`TransferReason`](crate::transfer::TransferReason) variants of
/// the same names. That correspondence is load-bearing rather than incidental:
/// the rule binding a derivation edge to the dual-signed transfer that consents
/// to it matches one against the other, so the two vocabularies must not drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum SecondLifeOperation {
    /// Art. 3(29) — preparing for re-use as defined in Art. 3, point (16), of
    /// Directive 2008/98/EC.
    PreparationForReuse,
    /// Art. 3(30) — a **waste** battery, or parts thereof, prepared so that it
    /// can be used for a different purpose than the one it was designed for.
    PreparationForRepurposing,
    /// Art. 3(31) — a battery that is **not** a waste battery, or parts
    /// thereof, used for a purpose other than the one it was designed for.
    Repurposing,
    /// Art. 3(32) — disassembly and evaluation of all cells and modules, and
    /// reuse of a number of them, to restore capacity to at least 90 % of the
    /// original rated capacity, for the same purpose as originally designed.
    Remanufacturing,
}

impl SecondLifeOperation {
    /// Every operation this build models, for exhaustive iteration.
    ///
    /// `SecondLifeOperation` is `#[non_exhaustive]`, so a consumer outside this
    /// crate cannot enumerate it, and one publishing an API description has to.
    /// See [`TransferReason::ALL`](crate::transfer::TransferReason::ALL) for the
    /// same contract.
    ///
    /// The list is closed by the article: Art. 77(7) names four operations and
    /// no delegated act may add a fifth without amending it.
    pub const ALL: &'static [Self] = &[
        Self::PreparationForReuse,
        Self::PreparationForRepurposing,
        Self::Repurposing,
        Self::Remanufacturing,
    ];

    /// The stable wire form, for payloads that carry the operation as a string.
    ///
    /// Spelled out rather than derived from `Serialize` so that renaming a
    /// variant cannot silently change what a registry receives — and so that it
    /// stays byte-identical to
    /// [`TransferReason::wire_str`](crate::transfer::TransferReason::wire_str)
    /// for the four names they share.
    pub fn wire_str(&self) -> &'static str {
        match self {
            Self::PreparationForReuse => "preparationForReuse",
            Self::PreparationForRepurposing => "preparationForRepurposing",
            Self::Repurposing => "repurposing",
            Self::Remanufacturing => "remanufacturing",
        }
    }
}

/// An upward edge to one predecessor a second-life unit derives from.
///
/// ✅ COMPLIANCE-PIN: EU 2023/1542, Art. 77(7) (OJ L 191, 28.7.2023, p. 73):
/// "Such battery shall have a new battery passport linked to the battery
/// passport **or passports** of the original battery **or batteries**."
///
/// Plural on both sides, which is why [`Passport::derived_from`](super::Passport::derived_from)
/// is a `Vec` and
/// not an `Option`. A stationary storage pack assembled from several retired EV
/// packs is the canonical second-life product, not a corner case, and the
/// single-predecessor shape this replaced could not represent it at all.
///
/// Wraps [`PassportRef`] rather than extending it: the reference stays a pure,
/// direction-neutral "where + pin" primitive, and the qualifier each direction
/// needs is added by the wrapper. Downward BOM edges wrap the same primitive
/// with their own qualifiers.
///
/// The edge is a *claim* until it is consented to. A hash-pin proves the target
/// has not been modified; it does not prove the target's operator agreed to the
/// relationship, and Art. 77(7) moves regulatory responsibility. The consent
/// artefact is the dual-signed
/// [`TransferRecord`](crate::transfer::TransferRecord); binding the two is a
/// cross-field rule, not a property of this type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DerivationRef {
    /// Where to fetch the predecessor, and the hash pinning its signed public
    /// view.
    pub reference: PassportRef,
    /// Which of the four Art. 77(7) operations produced this unit from that
    /// predecessor.
    ///
    /// Required, not optional. Art. 77(7) attaches different legal consequences
    /// to each operation, so an edge that does not say which one occurred does
    /// not record what the article asks for.
    pub operation: SecondLifeOperation,
}
