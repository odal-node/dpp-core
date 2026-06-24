//! Versioned schema registry for EU ESPR sector schemas.
//!
//! The registry ships with compile-time embedded schemas and supports runtime
//! registration of new versions ("hot-reload"). This lets a running platform
//! absorb delegated-act schema changes without recompilation.
//!
//! Embedded schemas come from `dpp-core/schemas/{sector}/v{version}.json`.
//! Runtime schemas are registered via [`VersionedSchemaRegistry::register`].

use semver::Version;

mod embedded;
mod registry;
#[cfg(test)]
mod tests;

pub use registry::VersionedSchemaRegistry;

// ─── Schema origin ──────────────────────────────────────────────────────────

/// Tracks whether a schema was baked in at compile time or loaded at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaOrigin {
    /// Compiled into the binary via `include_str!()`.
    Embedded,
    /// Loaded at runtime via [`VersionedSchemaRegistry::register`].
    Runtime,
}

// ─── Schema entry ───────────────────────────────────────────────────────────

/// A single (sector, version) → JSON schema mapping.
#[derive(Debug, Clone)]
pub struct SchemaEntry {
    pub sector: String,
    pub version: Version,
    pub json: String,
    pub origin: SchemaOrigin,
}

// ─── Registration error ─────────────────────────────────────────────────────

/// Errors returned by [`VersionedSchemaRegistry::register`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaRegistrationError {
    /// The provided JSON string is not valid JSON.
    InvalidJson(String),
    /// A schema for this (sector, version) already exists.
    /// Use `register_or_replace` to overwrite.
    AlreadyExists { sector: String, version: Version },
    /// The version string is not valid semver.
    InvalidVersion(String),
}

impl std::fmt::Display for SchemaRegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(msg) => write!(f, "invalid JSON schema: {msg}"),
            Self::AlreadyExists { sector, version } => {
                write!(f, "schema already exists for {sector} v{version}")
            }
            Self::InvalidVersion(v) => write!(f, "invalid semver version: {v}"),
        }
    }
}

impl std::error::Error for SchemaRegistrationError {}
