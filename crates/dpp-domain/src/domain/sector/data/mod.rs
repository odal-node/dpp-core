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
#[allow(clippy::module_inception)]
pub mod sector_data;
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
pub use sector_data::{SectorData, redact_sector_data};
pub use shared::{CriticalRawMaterial, SvhcSubstance};
pub use steel::SteelData;
pub use textile::{FibreEntry, TextileData};
pub use toy::ToyData;
pub use tyre::TyreData;
pub use unsold_goods::{UnsoldGoodsDestination, UnsoldGoodsReason, UnsoldGoodsReport};
