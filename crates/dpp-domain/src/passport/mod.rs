//! The `Passport` aggregate root and its unique identifier type.

pub mod id;

pub mod component;
#[cfg(test)]
mod component_tests;
pub mod derivation;
#[cfg(test)]
mod derivation_tests;
pub mod life_status;
#[cfg(test)]
mod life_status_tests;
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
mod retention_mutable_fields_tests;
#[cfg(test)]
pub(crate) mod tests;
#[cfg(test)]
mod validation_tests;
#[cfg(test)]
mod wire_keys_tests;

pub use crate::facility::FacilitySnapshot;
pub use crate::manufacturer::ManufacturerInfo;
pub use crate::material::MaterialEntry;
pub use component::{ComponentRef, Quantity};
pub use derivation::{DerivationRef, SecondLifeOperation};
pub use id::PassportId;
pub use life_status::LifeStatus;
pub use record::{
    PASSPORT_PROOF_FIELDS, PASSPORT_WIRE_KEYS, Passport, REMOVED_ENVELOPE_KEYS,
    RETENTION_MUTABLE_FIELDS,
};
pub use reference::PassportRef;
pub use view::PassportView;
