//! [`SchemaRegistrationError`] — why a runtime schema registration was refused.

use semver::Version;

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
