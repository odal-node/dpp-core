//! [`StateOfChargeReading`] — one recorded state-of-charge sample.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// One periodic state-of-charge observation — Annex XIII point 4(d).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct StateOfChargeReading {
    /// When the observation was taken.
    pub recorded_at: DateTime<Utc>,
    /// State of charge as a percentage of usable capacity.
    pub state_of_charge_pct: f64,
}
