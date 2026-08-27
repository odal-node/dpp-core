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
mod entry;
pub mod lens;
#[cfg(test)]
mod prose_act_reference_tests;
#[cfg(test)]
mod prose_citation_tests;
mod registration_error;
#[cfg(test)]
mod serialisation_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod validation_tests;
mod versioned;

pub use entry::{SchemaEntry, SchemaOrigin};
pub use lens::{DerivedView, Lens, LensError, LensRegistry, UpcastError};
pub use registration_error::SchemaRegistrationError;
pub use versioned::VersionedSchemaRegistry;
