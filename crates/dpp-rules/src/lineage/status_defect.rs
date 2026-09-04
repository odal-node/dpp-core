//! What the life-status consistency rule reports.

/// Why a passport's life status does not agree with its derivation edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StatusDefect<'a> {
    /// The passport claims `original` while also claiming to derive from a
    /// predecessor.
    ///
    /// An original unit is the one placed on the market, not the output of an
    /// operation performed on something else, so any derivation edge contradicts
    /// the claim. Carries the first offending operation rather than all of them:
    /// one is enough to show the contradiction, and the edge list is the
    /// caller's to report in full.
    OriginalIsDerived {
        /// The operation of the first derivation edge, in wire form.
        operation: &'a str,
    },
    /// The passport claims a second-life status that no derivation edge
    /// supports.
    ///
    /// Either the edges name a different operation, or there are none at all —
    /// which are the same defect from this rule's point of view, since both
    /// leave the claimed operation unrecorded. Whether the edge that *is* there
    /// carries its predecessor's consent is [`super::consent`]'s question.
    NoEdgeSupportsStatus {
        /// The status the passport claims, in wire form.
        status: &'a str,
    },
    /// The status is not one of the five values Annex XIII point 4(c)
    /// enumerates.
    ///
    /// Unreachable from core, whose `LifeStatus` is a closed enum, and the
    /// reason this variant exists anyway is the Wasm plugins: they hand this
    /// crate JSON string fields, so an unknown value is a shape a caller really
    /// can produce. Reported rather than ignored — a status outside the annex is
    /// an invented value, and the fail-open reading would be to treat it as
    /// consistent with anything.
    UnknownStatus {
        /// What the passport carried.
        status: &'a str,
    },
}
