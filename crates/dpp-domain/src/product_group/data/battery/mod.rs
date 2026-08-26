//! Battery (EU Battery Regulation 2023/1542).

mod category;
mod chemistry;
#[cfg(test)]
mod chemistry_tests;
mod data;
mod dynamic_performance;
mod environmental_reading;
mod expected_lifetime;
mod harmful_events;
mod hazardous_substance;
mod material_composition;
mod state_of_charge_reading;
mod state_of_health;
mod status;
mod temperature_range;
mod usage_history;

pub use category::BatteryType;
pub use chemistry::BatteryChemistry;
pub use data::BatteryData;
pub use dynamic_performance::DynamicPerformance;
pub use environmental_reading::EnvironmentalReading;
pub use expected_lifetime::ExpectedLifetime;
pub use harmful_events::HarmfulEvents;
pub use hazardous_substance::{HazardSymbol, HazardousSubstance};
pub use material_composition::MaterialComposition;
pub use state_of_charge_reading::StateOfChargeReading;
pub use state_of_health::StateOfHealth;
pub use status::BatteryStatus;
pub use temperature_range::TemperatureRange;
pub use usage_history::UsageHistory;
