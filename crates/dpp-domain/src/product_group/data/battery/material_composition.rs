//! [`MaterialComposition`] — one declared constituent material.

use serde::{Deserialize, Serialize};

/// Material composition entry for cathode, anode, or electrolyte.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MaterialComposition {
    /// Chemical name or formula, e.g. `"LiFePO4"`, `"graphite"`, `"LiPF6"`.
    pub name: String,
    /// Weight percentage in the component (0.0–100.0).
    pub weight_pct: f64,
    /// CAS Registry Number if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cas_number: Option<String>,
}
