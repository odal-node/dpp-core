//! Unsold Goods (EU ESPR, destruction ban — effective July 19, 2026).

use serde::{Deserialize, Serialize};

/// Destination category for unsold textile goods under EU ESPR Article 25 (Annex VII).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum UnsoldGoodsDestination {
    /// Donated to charity or social enterprise.
    Donation,
    /// Sent for material recycling.
    Recycling,
    /// Repurposed or upcycled within the supply chain.
    Repurposing,
    /// Returned to supplier for reuse.
    SupplierReturn,
    /// Destruction permitted under an approved exemption (requires justification).
    ExemptDestruction,
}

/// Reason category explaining why goods were unsold.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum UnsoldGoodsReason {
    EndOfSeason,
    QualityDefect,
    PackagingDefect,
    OverProduction,
    CustomerReturn,
    Other,
}

/// Unsold Goods Destruction Ban report — EU ESPR Article 25 (Annex VII), effective July 19, 2026.
///
/// Records the disposal of unsold textile goods. Destruction is banned unless
/// a specific exemption applies. All disposals must be reported in the DPP.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UnsoldGoodsReport {
    /// Reference period covered by this report (ISO 8601 date, e.g. `"2026-Q2"`).
    pub reporting_period: String,
    /// Total volume of unsold goods in kilograms.
    pub volume_kg: f64,
    /// Product category (e.g. `"apparel"`, `"footwear"`, `"home-textile"`).
    pub product_category: String,
    /// Reason the goods were unsold.
    pub reason: UnsoldGoodsReason,
    /// Destination / disposal method.
    pub destination: UnsoldGoodsDestination,
    /// If `destination` is `ExemptDestruction`, the mandatory justification text.
    pub destruction_justification: Option<String>,
    /// ISO 3166-1 alpha-2 country where disposal took place.
    pub country_of_disposal: String,
    /// Name of the disposal operator or charity recipient (for audit trail).
    pub operator_name: Option<String>,
}
