//! [`WasteTreatmentOperation`] — what the products were delivered to undergo.

use serde::{Deserialize, Serialize};

/// The waste treatment operation one discard was delivered for.
///
/// Annex I of Commission Implementing Regulation (EU) 2026/2 gives the operations
/// as columns, defined by Directive 2008/98/EC Art. 3: preparing for reuse (16),
/// recycling (17), recovery other than recycling (15) and disposal (19). ESPR
/// Art. 24(1)(c) asks for "the proportion of discarded products delivered, whether
/// directly or through a third party, to undergo" each of them.
///
/// A discard is **one delivery to one operation**; the proportions of a line are
/// derived from the weights of the discards that make it up
/// (see [`WasteTreatmentSplit`](crate::unsold_goods::WasteTreatmentSplit)). A
/// delivery that was treated two ways is two records.
///
/// [`Self::Unknown`] is an answer, not a gap: note (i) has the treatment "listed as
/// `unknown`" where the waste treatment operator could not say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum WasteTreatmentOperation {
    /// Preparing for reuse, Directive 2008/98/EC Art. 3(16). ESPR Art. 2(34) keeps
    /// discarding for the sole purpose of delivering products for it out of
    /// "destruction".
    PreparingForReuse,
    /// Recycling, Art. 3(17).
    Recycling,
    /// Other recovery, e.g. energy recovery, Art. 3(15).
    OtherRecovery,
    /// Disposal, Art. 3(19).
    Disposal,
    /// The operator could not establish the treatment.
    Unknown,
}
