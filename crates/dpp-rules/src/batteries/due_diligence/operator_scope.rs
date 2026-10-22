//! [`OperatorScope`] — the operator's declaration against Art. 47, first
//! paragraph.

/// The operator's declaration against Art. 47, first paragraph.
///
/// Wire values are `"subject"` and `"belowTurnoverThreshold"`. Neither names an
/// amount, on purpose: see the [module documentation](super).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorScope {
    /// The economic operator placing the battery is subject to Chapter VII.
    Subject,
    /// Art. 47, first paragraph, takes the operator out of Chapter VII: its net
    /// turnover in the financial year preceding the last was under the
    /// threshold, and it is not part of a group above it on a consolidated
    /// basis.
    BelowTurnoverThreshold,
}

impl OperatorScope {
    /// The wire spelling.
    #[must_use]
    pub const fn wire_str(self) -> &'static str {
        match self {
            Self::Subject => "subject",
            Self::BelowTurnoverThreshold => "belowTurnoverThreshold",
        }
    }

    /// Parse the wire spelling exactly. Anything else is `None`, which a caller
    /// must treat as undeclared rather than as either answer.
    #[must_use]
    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "subject" => Some(Self::Subject),
            "belowTurnoverThreshold" => Some(Self::BelowTurnoverThreshold),
            _ => None,
        }
    }
}
