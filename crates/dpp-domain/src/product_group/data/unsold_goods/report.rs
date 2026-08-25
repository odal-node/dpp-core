//! [`UnsoldGoodsReport`] — the whole Annex I disclosure.

use serde::{Deserialize, Serialize};

use super::entity::DisclosingEntity;
use super::financial_year::FinancialYear;
use super::line::DiscardedProductLine;

/// The disclosure of information on discarded unsold consumer products.
///
/// # What this is, and what it is not
///
/// This is **not a passport**. ESPR Arts. 24–25 impose a duty on an *economic
/// operator* over a *financial year*, disclosed on the operator's own website —
/// or, where it publishes sustainability reporting under Directive 2013/34/EU
/// Art. 19a or 29a, by a link to that report naming where the information sits
/// (Impl. Reg. (EU) 2026/2 Art. 2(2)). There is no product record anywhere in
/// it, which is why `unsold-goods` carries `PassportObligation::NotRequired`
/// while still being in force and determinable today.
///
/// It occupies a product-group slot for implementation convenience, not because
/// it is a product group.
///
/// # The format is prescribed
///
/// **Art. 2(1):** "The visual presentation and content of the disclosure … shall
/// comply with the format set out in **Annex I**." The structure below is that
/// table: a header identifying the undertaking and its financial year, a
/// repeating body of product lines, and two narrative rows.
///
/// Two formatting rules from Section 2 that a renderer must honour and a type
/// cannot: numbers carry **no separators**, and every figure is **rounded to the
/// nearest whole number**. See [`DiscardedQuantity`](super::line::DiscardedQuantity)
/// for the estimate marker.
///
/// # This shape replaced an invented one
///
/// The previous `UnsoldGoodsReport` predated both acts. It reported a free-text
/// quarter rather than a financial year, categorised by names like `"apparel"`
/// where Art. 3 requires CN codes, recorded a single destination where Annex I
/// requires a six-way percentage split, carried no unit count, no packaging
/// flag, no entity header and neither narrative row — and its reason list was
/// ours rather than the Regulation's. It is not a migration of that model; the
/// axes are different.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsoldGoodsReport {
    /// The undertaking making the disclosure — Annex I's first four rows.
    pub entity: DisclosingEntity,
    /// The financial year disclosed, by its own start and end dates.
    pub financial_year: FinancialYear,
    /// The body of the table. "Additional lines may be added as necessary"
    /// (Annex I, Section 2), and note (h) requires a separate line per reason
    /// within a category.
    pub lines: Vec<DiscardedProductLine>,
    /// Annex I note (i): measures taken to prevent destruction, which "shall
    /// include measures taken in the **preceding** financial year and must be
    /// based, where relevant, on the information on unsold consumer products
    /// destroyed in the past".
    pub measures_taken: String,
    /// Annex I note (j): measures planned, which "shall include measures for
    /// implementation in the future" and "in particular … specific measures
    /// necessary to prevent the destruction of the categories of products
    /// destroyed in the preceding financial year for the same reasons, and
    /// describe how the measures are expected to achieve that purpose".
    pub measures_planned: String,
}

impl UnsoldGoodsReport {
    /// Total weight discarded across every line, in kilogrammes.
    ///
    /// Sums the disclosed figures whether measured or estimated — the split
    /// between the two is a property of each line, and flattening it into one
    /// number here would assert a precision the lines do not have. Callers
    /// needing that distinction should read the lines.
    #[must_use]
    pub fn total_weight_kg(&self) -> u64 {
        self.lines.iter().map(|l| l.weight_kg.value).sum()
    }

    /// Total units discarded across every line.
    #[must_use]
    pub fn total_units(&self) -> u64 {
        self.lines.iter().map(|l| l.units_discarded.value).sum()
    }

    /// Whether any line reports an estimated figure, which Annex I requires be
    /// shown with `±` wherever it appears.
    #[must_use]
    pub fn contains_estimates(&self) -> bool {
        self.lines
            .iter()
            .any(|l| l.units_discarded.estimated || l.weight_kg.estimated)
    }
}

impl crate::product_group::payload::ProductGroupPayload for UnsoldGoodsReport {
    /// A disclosure covers a financial year across many products, so there is no
    /// single trade item number. The CN categories are on the lines.
    fn gtin(&self) -> Option<&str> {
        None
    }

    /// Arts. 24–25 define no model identifier.
    fn model_identifier(&self) -> Option<&str> {
        None
    }
}
