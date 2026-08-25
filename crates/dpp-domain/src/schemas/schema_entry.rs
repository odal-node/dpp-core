//! [`SchemaEntry`] — one (product group, version) → JSON schema mapping.

use semver::Version;

/// Tracks whether a schema was baked in at compile time or loaded at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaOrigin {
    /// Compiled into the binary via `include_str!()`.
    Embedded,
    /// Loaded at runtime via [`crate::schemas::VersionedSchemaRegistry::register`].
    Runtime,
}

/// A single (product group, version) → JSON schema mapping.
#[derive(Debug, Clone)]
pub struct SchemaEntry {
    pub product_group: String,
    pub version: Version,
    pub json: String,
    pub origin: SchemaOrigin,
}
