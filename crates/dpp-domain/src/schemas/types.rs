//! Supporting types for the versioned schema registry: origin tracking,
//! entries, and registration errors.

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

/// Errors returned by [`crate::schemas::VersionedSchemaRegistry::register`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaRegistrationError {
    /// The provided JSON string is not valid JSON.
    InvalidJson(String),
    /// A schema for this (product group, version) already exists.
    /// Use `register_or_replace` to overwrite.
    AlreadyExists {
        product_group: String,
        version: Version,
    },
    /// The version string is not valid semver.
    InvalidVersion(String),
}

impl std::fmt::Display for SchemaRegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(msg) => write!(f, "invalid JSON schema: {msg}"),
            Self::AlreadyExists {
                product_group,
                version,
            } => {
                write!(f, "schema already exists for {product_group} v{version}")
            }
            Self::InvalidVersion(v) => write!(f, "invalid semver version: {v}"),
        }
    }
}

impl std::error::Error for SchemaRegistrationError {}
