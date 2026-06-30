//! Cross-sector typed enumerations (3.2d).
//!
//! Shared enums referenced by more than one sector's data struct (chemistry,
//! production route, energy/carbon classes, LCA boundaries).

use serde::{Deserialize, Serialize};

/// Battery electrochemical chemistry with `#[serde(other)]` fallback for future types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BatteryChemistry {
    #[serde(rename = "LFP")]
    Lfp,
    #[serde(rename = "NMC")]
    Nmc,
    #[serde(rename = "NCA")]
    Nca,
    #[serde(rename = "LCO")]
    Lco,
    #[serde(rename = "NiMH")]
    NiMh,
    #[serde(rename = "NiCd")]
    NiCd,
    #[serde(rename = "lead-acid")]
    LeadAcid,
    #[serde(rename = "solid-state")]
    SolidState,
    /// Absorbs unknown chemistry codes on deserialization (forward compatibility).
    #[serde(other)]
    Other,
}

/// Battery type category per EU Battery Regulation 2023/1542 Art. 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum BatteryType {
    Portable,
    Industrial,
    Ev,
    Lmt,
    /// Starting, lighting, and ignition batteries.
    #[serde(rename = "starting-lighting-ignition")]
    Sli,
    #[serde(other)]
    Other,
}

/// Carbon footprint performance class (A–E) per EU Battery Regulation Art. 7(2).
/// Also used for electronics energy-rating analogues.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CarbonFootprintClass {
    A,
    B,
    C,
    D,
    E,
    #[serde(other)]
    Other,
}

/// EU energy label class per EU Energy Labelling Regulation 2017/1369 (A–G scale).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EnergyEfficiencyClass {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    #[serde(other)]
    Other,
}

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
