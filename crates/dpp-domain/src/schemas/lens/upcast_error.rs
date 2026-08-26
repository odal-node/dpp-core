//! [`UpcastError`] — why an upcast could not be produced.

use semver::Version;

use super::transform::LensError;

/// Why an upcast could not be produced. Never a silent identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpcastError {
    /// No chain of registered lenses bridges `from` → `to` for this product group.
    NoPath {
        product_group: String,
        from: Version,
        to: Version,
    },
    /// `to` is not newer than `from` — downcast is never supported.
    NotAnUpcast { from: Version, to: Version },
    /// A lens transform in the chain failed.
    Transform(LensError),
    /// A version string could not be parsed as semver.
    BadVersion(String),
}

impl std::fmt::Display for UpcastError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoPath {
                product_group,
                from,
                to,
            } => {
                write!(f, "no lens path for {product_group} {from} → {to}")
            }
            Self::NotAnUpcast { from, to } => {
                write!(
                    f,
                    "{to} is not an upcast of {from} — downcast is unsupported"
                )
            }
            Self::Transform(e) => write!(f, "{e}"),
            Self::BadVersion(v) => write!(f, "'{v}' is not a valid semver version"),
        }
    }
}

impl std::error::Error for UpcastError {}
