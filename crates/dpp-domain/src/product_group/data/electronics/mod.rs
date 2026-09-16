//! Electronics (Regulation (EU) 2023/1670 and the ESPR electronics act).

mod data;
mod device_type;
#[cfg(test)]
mod device_type_tests;
mod efficiency_class;
mod index_scope_exclusion;
mod priority_part_scores;
mod repairability_index;
#[cfg(test)]
mod repairability_index_tests;

pub use data::ElectronicsData;
pub use device_type::DeviceType;
pub use efficiency_class::EnergyEfficiencyClass;
pub use index_scope_exclusion::IndexScopeExclusion;
pub use priority_part_scores::{MAX_SCORE, MIN_SCORE, PriorityPartScores};
pub use repairability_index::RepairabilityIndexDeclaration;
