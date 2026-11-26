//! Why a metric has a value, or why it does not.

use chrono::NaiveDate;

/// The outcome of asking "which ruleset governs this product?".
///
/// Deliberately not `Option`. `None` collapses four legally distinct answers
/// into one, and the difference between them is what an operator is actually
/// owed: *"this regulation does not cover your product"*, *"it will cover it
/// from a date we can name"*, *"we are waiting on an act that has not been
/// adopted"* and *"the rule that covered it has been replaced"* carry different
/// obligations and different next actions. Reporting any of them as
/// non-compliance — or as a silent blank — misstates the operator's position.
#[derive(Debug, Clone, PartialEq)]
pub enum Assessability<T> {
    /// A governing ruleset was found.
    Assessed(T),
    /// A ruleset covers this product, from a known date that has not arrived.
    NotYetInForce {
        ruleset_id: &'static str,
        applies_from: NaiveDate,
    },
    /// A ruleset exists but the instrument that would date it has not entered
    /// into force, so it has no application date at all. Distinct from
    /// [`NotYetInForce`](Self::NotYetInForce): there is no date to wait for yet.
    Undetermined {
        ruleset_id: &'static str,
        empowerment: &'static str,
    },
    /// The ruleset that covered this product has ended.
    Expired {
        ruleset_id: &'static str,
        until: NaiveDate,
        /// Successor ruleset id from `regulatory_basis().superseded_by`, so a
        /// caller can follow the regulatory chain instead of dead-ending.
        superseded_by: Option<&'static str>,
    },
    /// No ruleset covers this product category at all.
    OutOfScope,
}

impl<T> Assessability<T> {
    /// The governing ruleset, discarding the reason when there isn't one.
    ///
    /// Use only where the caller genuinely has nothing to say about *why* —
    /// anything user-facing should match on the variant instead.
    #[must_use]
    pub fn assessed(self) -> Option<T> {
        match self {
            Self::Assessed(v) => Some(v),
            _ => None,
        }
    }

    /// Whether a governing ruleset was found.
    #[must_use]
    pub const fn is_assessed(&self) -> bool {
        matches!(self, Self::Assessed(_))
    }
}

#[cfg(test)]
mod tests;
