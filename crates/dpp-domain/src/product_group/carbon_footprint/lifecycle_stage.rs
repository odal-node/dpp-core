//! [`LifecycleStage`] — the LCA stage a figure covers.

use serde::{Deserialize, Serialize};

/// LCA lifecycle stage boundary for a carbon footprint declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum LifecycleStage {
    CradleToGate,
    CradleToGrave,
    CradleToCradle,
    GateToGrave,
    #[serde(other)]
    Other,
}
