//! [`ProductionRoute`] — how a material was produced.

use serde::{Deserialize, Serialize};

/// Steel and aluminium production route — determines carbon intensity basis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ProductionRoute {
    /// Integrated blast furnace / basic oxygen furnace (steel).
    BlastFurnace,
    /// Electric arc furnace (steel — typically secondary).
    ElectricArc,
    /// Direct reduced iron route (steel).
    DirectReduction,
    /// Primary Hall-Héroult electrolysis (aluminium).
    Primary,
    /// Secondary recycled route (aluminium).
    SecondaryRecycled,
    Mixed,
    #[serde(other)]
    Other,
}
