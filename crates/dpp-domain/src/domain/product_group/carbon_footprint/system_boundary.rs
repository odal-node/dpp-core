//! [`SystemBoundary`] — the LCA system boundary standard a figure used.

use serde::{Deserialize, Serialize};

/// LCA system-boundary standard referenced in a carbon footprint declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SystemBoundary {
    #[serde(rename = "EN-15804")]
    En15804,
    #[serde(rename = "ISO-14044")]
    Iso14044,
    #[serde(rename = "GHG-protocol")]
    GhgProtocol,
    #[serde(other)]
    Other,
}
