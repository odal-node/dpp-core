//! [`TemperatureRange`] — the operating window a battery declares.

use serde::{Deserialize, Serialize};

/// A temperature range in degrees Celsius.
///
/// Annex XIII asks for a range in three places — attached to the voltage
/// figures at point 1(h), to the power limits at 1(i), and standing alone at
/// 1(l) for the not-in-use case. One type rather than six loose bounds, so a
/// range cannot be half-declared.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct TemperatureRange {
    /// Lower bound in °C.
    pub min_c: f64,
    /// Upper bound in °C.
    pub max_c: f64,
}
