//! Cross-product group typed enumerations (3.2d).
//!
//! Shared enums referenced by more than one product group's data struct (chemistry,
//! production route, energy/carbon classes, LCA boundaries).

mod battery_chemistry;
#[cfg(test)]
mod battery_chemistry_tests;
mod battery_status;
mod battery_type;
mod carbon_footprint_class;
#[cfg(test)]
mod carbon_footprint_class_tests;
mod device_type;
mod energy_efficiency_class;
mod lifecycle_stage;
mod production_route;
mod system_boundary;

pub use battery_chemistry::BatteryChemistry;
pub use battery_status::BatteryStatus;
pub use battery_type::BatteryType;
pub use carbon_footprint_class::{CarbonFootprintClass, CarbonFootprintClassError};
pub use device_type::DeviceType;
pub use energy_efficiency_class::EnergyEfficiencyClass;
pub use lifecycle_stage::LifecycleStage;
pub use production_route::ProductionRoute;
pub use system_boundary::SystemBoundary;
