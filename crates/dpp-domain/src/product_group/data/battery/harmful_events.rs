//! [`HarmfulEvents`] — counts of events known to shorten battery life.

use serde::{Deserialize, Serialize};

/// Harmful events tracked under Annex VII Part B item 4.
///
/// The annex says *"the tracking of harmful events, **such as** the number of
/// deep discharge events, time spent in extreme temperatures, time spent
/// charging in extreme temperatures"*. "Such as" makes that list illustrative,
/// not closed — so every field here is optional, and an implementation tracking
/// a further event type is conforming, not extending.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct HarmfulEvents {
    /// Number of deep discharge events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deep_discharge_events: Option<u32>,
    /// Cumulative hours spent outside the battery's rated temperature range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hours_in_extreme_temperature: Option<f64>,
    /// Cumulative hours spent *charging* outside that range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hours_charging_in_extreme_temperature: Option<f64>,
}
