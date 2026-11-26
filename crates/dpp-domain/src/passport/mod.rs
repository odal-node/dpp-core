//! The `Passport` aggregate root and its unique identifier type.

pub mod id;
#[cfg(test)]
pub(crate) mod tests;

pub mod carrier;
pub mod component;
pub mod derivation;
pub mod life_status;
pub mod record;
pub mod reference;
pub mod view;

pub use crate::facility::FacilitySnapshot;
pub use crate::manufacturer::ManufacturerInfo;
pub use crate::material::MaterialEntry;
pub use carrier::{CarrierLevel, CarrierQualifier};
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
