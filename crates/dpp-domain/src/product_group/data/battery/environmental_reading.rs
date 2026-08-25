//! [`EnvironmentalReading`] — one recorded temperature/environment sample.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// One periodic environmental observation — Annex XIII point 4(d).
///
/// The annex names temperature explicitly and leaves the rest of "operating
/// environmental conditions" open, so temperature is the one typed member and
/// anything further is recorded as a note rather than invented as a field.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct EnvironmentalReading {
    /// When the observation was taken.
    pub recorded_at: DateTime<Utc>,
    /// Temperature in °C.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature_c: Option<f64>,
    /// Any further condition the annex leaves unenumerated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}
