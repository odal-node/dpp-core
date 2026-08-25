//! Versioned schema registry for EU ESPR product group schemas.
//!
//! The registry ships with compile-time embedded schemas and supports runtime
//! registration of new versions ("hot-reload"). This lets a running platform
//! absorb delegated-act schema changes without recompilation.
//!
//! Embedded schemas come from `dpp-core/schemas/{product group}/v{version}.json`.
//! Runtime schemas are registered via [`VersionedSchemaRegistry::register`].

#[cfg(test)]
mod conformance_shape_tests;
#[cfg(test)]
mod conformance_tests;
mod embedded;
pub mod lens;
mod registration_error;
mod schema_entry;
#[cfg(test)]
mod serialisation_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod validation_tests;
mod versioned;

pub use lens::{DerivedView, Lens, LensError, LensRegistry, UpcastError};
pub use registration_error::SchemaRegistrationError;
pub use schema_entry::{SchemaEntry, SchemaOrigin};
pub use versioned::VersionedSchemaRegistry;
