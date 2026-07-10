//! [`CatalogError`] — errors from runtime catalog registration.

/// Errors from runtime catalog registration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CatalogError {
    /// A descriptor for this key already exists.
    AlreadyExists(String),
    /// `current_schema_version` is not a valid semver string.
    InvalidSchemaVersion { key: String, version: String },
    /// `current_schema_version` is not listed in `schema_versions`.
    CurrentVersionNotListed { key: String, version: String },
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyExists(key) => write!(f, "sector '{key}' already in catalog"),
            Self::InvalidSchemaVersion { key, version } => write!(
                f,
                "sector '{key}' currentSchemaVersion '{version}' is not valid semver"
            ),
            Self::CurrentVersionNotListed { key, version } => write!(
                f,
                "sector '{key}' currentSchemaVersion '{version}' is not in its schemaVersions list"
            ),
        }
    }
}

impl std::error::Error for CatalogError {}
