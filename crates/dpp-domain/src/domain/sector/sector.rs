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

    /// The serde wire tag for this sector (its `camelCase` JSON discriminant),
    /// e.g. `"unsoldGoods"` — distinct from [`Self::catalog_key`], which is
    /// kebab-case (`"unsold-goods"`) and used by the schema registry/catalog.
    ///
    /// Equivalent to `serde_json::to_value(self)` but without the allocation
    /// and `Value` round trip.
    pub const fn wire_str(&self) -> &'static str {
        match self {
            Self::Battery => "battery",
            Self::Textile => "textile",
            Self::UnsoldGoods => "unsoldGoods",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_str_matches_serde_serialization() {
        for sector in [
            Sector::Battery,
            Sector::Textile,
            Sector::UnsoldGoods,
            Sector::Steel,
            Sector::Electronics,
            Sector::Construction,
            Sector::Tyre,
            Sector::Toy,
            Sector::Aluminium,
            Sector::Furniture,
            Sector::Detergent,
            Sector::Other,
        ] {
            let serialized = serde_json::to_value(&sector).unwrap();
            assert_eq!(
                serialized.as_str().unwrap(),
                sector.wire_str(),
                "wire_str() disagrees with serde for {sector:?}"
            );
        }
    }
}
