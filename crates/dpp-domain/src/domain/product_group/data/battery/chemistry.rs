//! [`BatteryChemistry`] — the cell chemistry a battery declares.

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

impl BatteryChemistry {
    /// The serde wire tag for this chemistry code, e.g. `"LFP"`, `"lead-acid"`.
    /// Equivalent to `serde_json::to_value(self)` but without the allocation
    /// and `Value` round trip.
    pub const fn wire_str(&self) -> &'static str {
        match self {
            Self::Lfp => "LFP",
            Self::Nmc => "NMC",
            Self::Nca => "NCA",
            Self::Lco => "LCO",
            Self::NiMh => "NiMH",
            Self::NiCd => "NiCd",
            Self::LeadAcid => "lead-acid",
            Self::SolidState => "solid-state",
            Self::Other => "Other",
        }
    }
}
