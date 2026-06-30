//! Per-sector typed payloads and the discriminated [`SectorData`] union.
//!
//! One file per sector, mirroring the per-sector catalog manifests and JSON
//! schemas 1:1. [`shared`] holds payload structs used by more than one sector.

pub mod aluminium;
pub mod battery;
pub mod construction;
pub mod detergent;
pub mod electronics;
pub mod furniture;
pub mod shared;
pub mod steel;
pub mod textile;
pub mod toy;
pub mod tyre;
pub mod unsold_goods;

pub use aluminium::AluminiumData;
pub use battery::{BatteryData, MaterialComposition};
pub use construction::ConstructionData;
pub use detergent::{DetergentData, SurfactantEntry};
pub use electronics::ElectronicsData;
pub use furniture::FurnitureData;
pub use shared::{CriticalRawMaterial, SvhcSubstance};
pub use steel::SteelData;
pub use textile::{FibreEntry, TextileData};
pub use toy::ToyData;
pub use tyre::TyreData;
pub use unsold_goods::{UnsoldGoodsDestination, UnsoldGoodsReason, UnsoldGoodsReport};

use serde::{Deserialize, Serialize};

use crate::domain::sector::Sector;

/// Typed, sector-specific DPP data — replaces the opaque `compliance_data: Value`.
///
/// Serialises as an internally-tagged object where `"sector"` is the
/// discriminant field, e.g.:
/// ```json
/// { "sector": "battery", "gtin": "09506000134352", "nominalVoltageV": 3.2, ... }
/// ```
/// ```json
/// { "sector": "textile", "fibreComposition": [...], "countryOfManufacturing": "BD" }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "sector", rename_all = "camelCase")]
#[non_exhaustive]
pub enum SectorData {
    Battery(BatteryData),
    Textile(TextileData),
    UnsoldGoods(UnsoldGoodsReport),
    Steel(SteelData),
    Electronics(ElectronicsData),
    Construction(ConstructionData),
    Tyre(TyreData),
    Toy(ToyData),
    Aluminium(AluminiumData),
    Furniture(FurnitureData),
    Detergent(DetergentData),
    Other(serde_json::Value),
}

impl SectorData {
    /// Returns the `Sector` discriminant for this data.
    pub fn sector(&self) -> Sector {
        match self {
            SectorData::Battery(_) => Sector::Battery,
            SectorData::Textile(_) => Sector::Textile,
            SectorData::UnsoldGoods(_) => Sector::UnsoldGoods,
            SectorData::Steel(_) => Sector::Steel,
            SectorData::Electronics(_) => Sector::Electronics,
            SectorData::Construction(_) => Sector::Construction,
            SectorData::Tyre(_) => Sector::Tyre,
            SectorData::Toy(_) => Sector::Toy,
            SectorData::Aluminium(_) => Sector::Aluminium,
            SectorData::Furniture(_) => Sector::Furniture,
            SectorData::Detergent(_) => Sector::Detergent,
            SectorData::Other(_) => Sector::Other,
        }
    }
}

/// Serialize `data` to a JSON object and strip any top-level field whose
/// required access tier exceeds `viewer_tier`.
///
/// `descriptor.access_tiers` maps camelCase JSON field names to the minimum
/// [`crate::domain::identity::AccessTier`] a viewer must hold to see that field.
/// Fields not listed in the map are always retained (default: Public).
///
/// Returns a `serde_json::Value::Object` with redacted fields removed.
/// Returns `serde_json::Value::Null` if serialization fails.
pub fn redact_sector_data(
    data: &SectorData,
    viewer_tier: crate::domain::identity::AccessTier,
    descriptor: &crate::catalog::SectorDescriptor,
) -> serde_json::Value {
    let mut value = match serde_json::to_value(data) {
        Ok(v) => v,
        Err(_) => return serde_json::Value::Null,
    };
    if let Some(obj) = value.as_object_mut() {
        obj.retain(|key, _| match descriptor.access_tiers.get(key) {
            Some(&required) => viewer_tier >= required,
            None => true,
        });
    }
    value
}
