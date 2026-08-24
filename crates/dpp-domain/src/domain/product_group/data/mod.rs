//! Per-product group typed payloads and the discriminated [`ProductGroupData`] union.
//!
//! One file per product group, mirroring the per-product group catalog manifests and JSON
//! schemas 1:1. [`shared`] holds payload structs used by more than one product group.

pub mod aluminium;
pub mod battery;
pub mod construction;
pub mod detergent;
pub mod electronics;
pub mod furniture;
pub mod mattress;
#[allow(clippy::module_inception)]
pub mod product_group_data;
pub mod shared;
pub mod steel;
pub mod textile;
pub mod toy;
pub mod tyre;
pub mod unsold_goods;

pub use aluminium::AluminiumData;
pub use battery::{
    BatteryData, DynamicPerformance, EnvironmentalReading, ExpectedLifetime, HarmfulEvents,
    HazardSymbol, HazardousSubstance, MaterialComposition, StateOfChargeReading, StateOfHealth,
    TemperatureRange, UsageHistory,
};
pub use construction::ConstructionData;
pub use detergent::{DetergentData, SurfactantEntry};
pub use electronics::ElectronicsData;
pub use furniture::FurnitureData;
pub use mattress::MattressData;
pub use product_group_data::{ProductGroupData, redact_product_group_data};
pub use shared::{CriticalRawMaterial, SvhcSubstance};
pub use steel::SteelData;
pub use textile::{FibreEntry, TextileData};
pub use toy::ToyData;
pub use tyre::TyreData;
pub use unsold_goods::{UnsoldGoodsDestination, UnsoldGoodsReason, UnsoldGoodsReport};
