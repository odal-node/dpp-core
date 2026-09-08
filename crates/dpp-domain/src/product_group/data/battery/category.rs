//! [`BatteryType`](crate::product_group::BatteryType) — the Regulation (EU) 2023/1542 Art. 3 battery category.

use serde::{Deserialize, Serialize};

/// Battery category per EU Battery Regulation 2023/1542 Art. 1(3).
///
/// **Deliberately no `#[serde(other)]` catch-all.** Art. 1(3) is a closed
/// enumeration of exactly five categories, and its second subparagraph gives a
/// tie-break rule — "the category to which the strictest requirements
/// apply" — that only functions over a closed set. An unrecognised value is
/// therefore a reason to reject the record, not to absorb and lose it: the
/// same defect class already fixed for [`CarbonFootprintClass`](crate::product_group::CarbonFootprintClass), in a field
/// that Annex VI Part A point 2 (via Annex XIII point 1(a)) makes mandatory
/// public passport content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum BatteryType {
    Portable,
    Industrial,
    Ev,
    Lmt,
    /// Starting, lighting, and ignition batteries.
    #[serde(rename = "starting-lighting-ignition")]
    Sli,
}

impl BatteryType {
    /// The serde wire tag for this category, e.g. `"ev"`,
    /// `"starting-lighting-ignition"`.
    ///
    /// Equivalent to `serde_json::to_value(self)` but without the allocation
    /// and `Value` round trip. `battery_type_wire_str_matches_serde` pins the
    /// two together.
    pub const fn wire_str(&self) -> &'static str {
        match self {
            Self::Portable => "portable",
            Self::Industrial => "industrial",
            Self::Ev => "ev",
            Self::Lmt => "lmt",
            Self::Sli => "starting-lighting-ignition",
        }
    }
}
