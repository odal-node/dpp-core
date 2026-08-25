//! [`DynamicPerformance`] — Annex XIII point 2 dynamic performance data.

use serde::{Deserialize, Serialize};

/// Measured performance and durability of one physical battery — Annex XIII
/// point 4(a).
///
/// # Why this is a block and not ten more fields on [`BatteryData`](crate::product_group::BatteryData)
///
/// Point 4 describes **an individual battery**; points 1 to 3 describe a
/// **model**. The Commission's own data-point guidance makes the pairing
/// explicit rather than implicit — its entry for `ratedCapacityAh` here reads
/// *"same as data point number 11 (capacity), but now dynamic"* — so the same
/// quantity is deliberately carried twice, once as declared and once as
/// measured. Flattening these onto `BatteryData` would put the two readings
/// side by side distinguished only by name, and would let a filer put a
/// measured value in a declared field. Keeping the block separate makes the
/// distinction structural, and lets one `individual` disclosure entry cover
/// the whole set — the same shape [`StateOfHealth`](crate::product_group::StateOfHealth) and [`ExpectedLifetime`](crate::product_group::ExpectedLifetime)
/// already use.
///
/// # Optionality
///
/// Every field is `Option`. The guidance marks this set mandatory for EV and
/// LMT batteries but *"if applicable"* for industrial ones, and marks round
/// trip efficiency and its fade *"where applicable"* for all three. Which
/// fields a given battery owes is a category-conditional rule, not a schema
/// constraint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct DynamicPerformance {
    /// Rated capacity in ampere-hours, measured. The dynamic counterpart of
    /// [`BatteryData::nominal_capacity_ah`](crate::product_group::BatteryData::nominal_capacity_ah).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rated_capacity_ah: Option<f64>,
    /// Capacity fade, as a percentage of the original rated capacity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity_fade_pct: Option<f64>,
    /// Power, in watts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_w: Option<f64>,
    /// Power fade, as a percentage of the original power.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_fade_pct: Option<f64>,
    /// Internal resistance, in milliohms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_resistance_mohm: Option<f64>,
    /// Internal resistance increase, as a percentage of the original.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_resistance_increase_pct: Option<f64>,
    /// Energy round trip efficiency, as a percentage. *"Where applicable"* for
    /// every category, unlike the fields above.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round_trip_efficiency_pct: Option<f64>,
    /// Energy round trip efficiency fade, as a percentage. *"Where
    /// applicable"*.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round_trip_efficiency_fade_pct: Option<f64>,
    /// Expected lifetime under the reference conditions the battery was
    /// designed for, in cycles — *"except for non-cycle applications"*. The
    /// measured counterpart of [`BatteryData::expected_lifetime_cycles`](crate::product_group::BatteryData::expected_lifetime_cycles).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_lifetime_cycles: Option<u32>,
    /// The same expectation in **calendar years**, which the annex lists as a
    /// separate data point rather than a unit conversion: a battery can have a
    /// calendar-life expectation and no meaningful cycle count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_lifetime_years: Option<f64>,
}
