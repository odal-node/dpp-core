//! The `Passport` aggregate root and its unique identifier type.

pub mod facility;
pub mod id;
pub mod manufacturer;
pub mod material;

pub mod record;
pub mod reference;
#[cfg(test)]
mod reference_tests;
pub mod view;

#[cfg(test)]
mod from_stored_tests;
#[cfg(test)]
mod publish_gate_tests;
#[cfg(test)]
mod redaction_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod validation_tests;

pub use facility::FacilitySnapshot;
pub use id::PassportId;
pub use manufacturer::ManufacturerInfo;
pub use material::MaterialEntry;
pub use record::{PASSPORT_WIRE_KEYS, Passport, RETENTION_MUTABLE_FIELDS};
pub use reference::PassportRef;
pub use view::PassportView;
