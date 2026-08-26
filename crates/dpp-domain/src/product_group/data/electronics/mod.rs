//! Electronics (Regulation (EU) 2023/1670 and the ESPR electronics act).

mod data;
mod device_type;
mod efficiency_class;

pub use data::ElectronicsData;
pub use device_type::DeviceType;
pub use efficiency_class::EnergyEfficiencyClass;
