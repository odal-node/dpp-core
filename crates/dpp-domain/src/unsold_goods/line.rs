//! [`DiscardedProductLine`] — one row of the Annex I disclosure table, and the
//! [`DiscardedQuantity`] that carries whether a figure was measured or estimated.

use serde::{Deserialize, Serialize};

use super::reason::DiscardReason;
use super::treatment::WasteTreatmentSplit;
use crate::identifier::cn_category::CnCategory;

/// A whole-number quantity, and whether it was counted or estimated.
///
/// **Annex I, Section 2:** "The format of the numbers shall not include
/// separators and the information shall be **rounded to the nearest whole
/// number**" — hence `u64` and not a float.
///
/// Notes (f) and (g) allow either figure to be derived from the other — units
/// estimated from an accurately determined weight, or weight from an accurate
/// count — and then require that the estimate be marked: "Where estimates are
/// used, this should be clarified by accompanying the disclosed value with
/// `±`."
///
/// A struct rather than two loose fields so a value cannot exist without its
/// provenance, and a provenance flag cannot be left orphaned when the value
/// moves. Same reasoning as `ObligationDate` in the instrument catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscardedQuantity {
    /// The figure, rounded to a whole number.
    pub value: u64,
    /// Whether `value` is an estimate. Renders with the `±` Annex I requires.
    #[serde(default)]
    pub estimated: bool,
}

impl DiscardedQuantity {
    /// A counted figure.
    #[must_use]
    pub fn measured(value: u64) -> Self {
        Self {
            value,
            estimated: false,
        }
    }

    /// An estimated figure — displays with the `±` marker.
    #[must_use]
    pub fn estimated(value: u64) -> Self {
        Self {
            value,
            estimated: true,
        }
    }
}

impl std::fmt::Display for DiscardedQuantity {
    /// Renders as Annex I requires: no separators, and `±` where estimated.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.estimated {
            write!(f, "±{}", self.value)
        } else {
            write!(f, "{}", self.value)
        }
    }
}

/// One row of the Annex I table: a product category, discarded in one financial
/// year, for one reason.
///
/// # One line per reason, not per category
///
/// **Annex I note (h):** "If units of the same product category are discarded
/// for **different reasons**, a separate line is necessary for each reason,
/// indicating the number and weight of units for each reason." So a category may
/// legitimately appear on several lines, and [`Self::reason`] is singular by
/// design rather than a set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscardedProductLine {
    /// The CN chapter or heading(s) this line covers — note (d), and Art. 3 for
    /// which depth applies.
    ///
    /// Plural because note (f) allows it: "Multiple items sold together, such as
    /// an electric drill with drill bits, cosmetic kits or first aid kits, may be
    /// considered as one unit and may, where appropriate, **indicate more than
    /// one CN code**."
    pub cn_categories: Vec<CnCategory>,
    /// Note (e): "established on the basis of the combined nomenclature … or a
    /// more detailed description".
    pub description: String,
    /// Note (f): total units discarded in the period, for this category.
    pub units_discarded: DiscardedQuantity,
    /// Note (g): combined weight of those units, in kilogrammes.
    pub weight_kg: DiscardedQuantity,
    /// Whether packaging is included in [`Self::weight_kg`] — its own column in
    /// Annex I, because the answer changes what the weight means.
    pub packaging_included: bool,
    /// Note (h), and the closed list in Del. Reg. (EU) 2026/296 Art. 2.
    pub reason: DiscardReason,
    /// A more detailed explanation, which note (h) permits alongside the reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_detail: Option<String>,
    /// Note (i): where the line actually went, as percentages of weight.
    pub treatment: WasteTreatmentSplit,
}
