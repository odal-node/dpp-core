//! The carbon footprint declaration and the vocabulary it is stated in.
//!
//! [`CarbonFootprint`] carries the figure; [`LifecycleStage`] and
//! [`SystemBoundary`] say what it covers; [`CarbonFootprintClass`] is the
//! performance label a manufacturer assigns. They live together because they
//! are one concept, and a reader who finds one wants the others.

mod class;
#[cfg(test)]
mod class_tests;
mod error;
mod footprint;
mod lifecycle_stage;
mod system_boundary;

pub use class::CarbonFootprintClass;
pub use error::CarbonFootprintClassError;
pub use footprint::CarbonFootprint;
pub use lifecycle_stage::LifecycleStage;
pub use system_boundary::SystemBoundary;
