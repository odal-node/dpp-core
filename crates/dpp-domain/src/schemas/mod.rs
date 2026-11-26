//! Versioned schema registry for EU ESPR product group schemas.
//!
//! The registry ships with compile-time embedded schemas and supports runtime
//! registration of new versions ("hot-reload"). This lets a running platform
//! absorb delegated-act schema changes without recompilation.
//!
//! Embedded schemas come from `dpp-core/schemas/{product group}/v{version}.json`.
//! Runtime schemas are registered via [`VersionedSchemaRegistry::register`].

mod change;
pub mod citation;
pub mod diff;
mod embedded;
mod entry;
pub mod lens;
mod registration_error;
#[cfg(test)]
mod tests;
mod versioned;

pub use change::{ChangeKind, PropertyChange};
pub use diff::{SchemaDiff, diff_schemas};
pub use entry::{SchemaEntry, SchemaOrigin};
pub use lens::{DerivedView, Lens, LensError, LensRegistry, UpcastError};
pub use registration_error::SchemaRegistrationError;
pub use versioned::VersionedSchemaRegistry;
