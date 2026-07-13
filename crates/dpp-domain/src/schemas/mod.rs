//! Versioned schema registry for EU ESPR sector schemas.
//!
//! The registry ships with compile-time embedded schemas and supports runtime
//! registration of new versions ("hot-reload"). This lets a running platform
//! absorb delegated-act schema changes without recompilation.
//!
//! Embedded schemas come from `dpp-core/schemas/{sector}/v{version}.json`.
//! Runtime schemas are registered via [`VersionedSchemaRegistry::register`].

mod embedded;
pub mod lens;
#[cfg(test)]
mod tests;
mod types;
mod versioned;

pub use lens::{DerivedView, Lens, LensError, LensRegistry, UpcastError};
pub use types::{SchemaEntry, SchemaOrigin, SchemaRegistrationError};
pub use versioned::VersionedSchemaRegistry;
