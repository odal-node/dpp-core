//! [`Sector`] — the EU ESPR dispatch discriminant.

use serde::{Deserialize, Serialize};

/// EU ESPR product sector — determines which delegated act schema applies.
///
/// Used by the compliance infrastructure to dispatch to the correct
/// `ComplianceStrategy` implementation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum Sector {
    Battery,
    Textile,
    UnsoldGoods,
    Steel,
    Electronics,
    Construction,
    Tyre,
    Toy,
    Aluminium,
    Furniture,
    Detergent,
    Other,
}

impl Sector {
    /// Minimum data retention period in years as required by the applicable
    /// EU delegated act.  The Battery Regulation (2023/1542) mandates ≥ 10
    /// years after end-of-life.  Other sectors default to 10 years pending
    /// their respective delegated acts.
    pub const fn minimum_retention_years(&self) -> u32 {
        match self {
            Self::Battery => 10,
            Self::Textile | Self::UnsoldGoods => 10,
            Self::Steel => 10,
            Self::Electronics => 10,
            Self::Construction => 10,
            Self::Tyre => 10,
            Self::Toy => 10,
            Self::Aluminium => 10,
            Self::Furniture => 10,
            Self::Detergent => 10,
            Self::Other => 10,
        }
    }

    /// Canonical sector key used by the schema registry and the `SectorCatalog`.
    ///
    /// This is the one true spelling (kebab-case where needed), distinct from
    /// the enum's camelCase serde tag — e.g. `UnsoldGoods` serialises as
    /// `"unsoldGoods"` but its catalog/registry key is `"unsold-goods"`.
    pub const fn catalog_key(&self) -> &'static str {
        match self {
            Self::Battery => "battery",
            Self::Textile => "textile",
            Self::UnsoldGoods => "unsold-goods",
            Self::Steel => "steel",
            Self::Electronics => "electronics",
            Self::Construction => "construction",
            Self::Tyre => "tyre",
            Self::Toy => "toy",
            Self::Aluminium => "aluminium",
            Self::Furniture => "furniture",
            Self::Detergent => "detergent",
            Self::Other => "other",
        }
    }
}
