//! [`CatalogError`] — errors from runtime catalog registration.

/// Errors from runtime catalog registration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CatalogError {
    /// A descriptor for this key already exists.
    AlreadyExists(String),
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyExists(key) => write!(f, "sector '{key}' already in catalog"),
        }
    }
}

impl std::error::Error for CatalogError {}
