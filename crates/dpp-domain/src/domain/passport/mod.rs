//! The `Passport` aggregate root and its unique identifier type.

pub mod facility;
pub mod id;
pub mod manufacturer;
pub mod material;
#[allow(clippy::module_inception)]
pub mod passport;
pub mod reference;
#[cfg(test)]
mod reference_tests;
pub mod view;

#[cfg(test)]
mod tests;

pub use facility::FacilitySnapshot;
pub use id::PassportId;
pub use manufacturer::ManufacturerInfo;
pub use material::MaterialEntry;
pub use passport::{PASSPORT_WIRE_KEYS, Passport, RETENTION_MUTABLE_FIELDS};
pub use reference::PassportRef;
pub use view::PassportView;
