//! [`DiscardedWeight`] — what one discard weighed, and whether it was weighed.

use serde::{Deserialize, Serialize};

/// The weight of one discard, in grams, and whether it is an estimate.
///
/// # Grams, where the disclosure counts kilogrammes
///
/// **Annex I, Section 2** has every figure "rounded to the nearest whole number",
/// and the weight column is in kilogrammes. That rounding belongs to the line, once,
/// after the weights are summed: rounding each discard to whole kilogrammes first
/// turns a 400 g parcel into nothing and lets the total drift by up to half a
/// kilogramme for every record. So a record keeps the finer unit and the
/// aggregation does the rounding.
///
/// Note (g) lets the weight be "estimated on the basis of the accurately determined
/// number of units", and requires an estimate to be marked, so the marker travels
/// with the figure as it does on [`DiscardedQuantity`](crate::unsold_goods::DiscardedQuantity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscardedWeight {
    /// The weight, in grams.
    pub grams: u64,
    /// Whether `grams` is an estimate. A line made from any estimated weight is
    /// itself disclosed as an estimate.
    #[serde(default)]
    pub estimated: bool,
}

impl DiscardedWeight {
    /// A weighed figure.
    #[must_use]
    pub fn measured(grams: u64) -> Self {
        Self {
            grams,
            estimated: false,
        }
    }

    /// An estimated figure.
    #[must_use]
    pub fn estimated(grams: u64) -> Self {
        Self {
            grams,
            estimated: true,
        }
    }
}
