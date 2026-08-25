//! [`UsageHistory`] — Annex XIII point 4 individual-item usage data.

use serde::{Deserialize, Serialize};

use super::environmental_reading::EnvironmentalReading;
use super::state_of_charge_reading::StateOfChargeReading;

/// Recorded use history of one physical battery — Annex XIII point 4(d).
///
/// Every item in 4(d) is *"if applicable"* for all three battery categories,
/// so nothing here is ever required by the schema.
///
/// **`negativeEvents` deliberately does not duplicate [`HarmfulEvents`](crate::product_group::HarmfulEvents).**
/// Annex VII Part B item 4 already requires harmful-event tracking as part of
/// the expected-lifetime parameter set, and this annex asks for the same
/// underlying facts under a different heading. Where a battery reports Part B
/// figures, [`ExpectedLifetime::harmful_events`](crate::product_group::ExpectedLifetime::harmful_events) is the structured home and
/// this field carries what does not fit it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct UsageHistory {
    /// Number of charging and discharging cycles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charge_discharge_cycles: Option<u32>,
    /// Negative events, such as accidents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_events: Option<Vec<String>>,
    /// Periodically recorded operating environmental conditions, including
    /// temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operating_conditions: Option<Vec<EnvironmentalReading>>,
    /// Periodically recorded state of charge, as a percentage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_of_charge: Option<Vec<StateOfChargeReading>>,
}
