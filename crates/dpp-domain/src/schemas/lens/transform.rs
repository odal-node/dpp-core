//! [`Lens`] — a single-hop, pure upcast transform, and the error it may refuse with.

use semver::Version;
use serde_json::Value;

/// A single-hop, pure upcast transform between two versions of one product group's
/// schema.
pub struct Lens {
    pub product_group: String,
    pub from: Version,
    pub to: Version,
    /// Whether the transform may drop or default source information. An honest
    /// lens over a purely additive schema change is `false`; one that must
    /// discard a removed field is `true`.
    pub lossy: bool,
    /// The regulatory change or rationale this lens bridges.
    pub note: &'static str,
    /// Pure transform, total over inputs that validate against `from`.
    pub(super) transform: fn(&Value) -> Result<Value, LensError>,
}

impl Lens {
    #[must_use]
    pub fn new(
        product_group: impl Into<String>,
        from: Version,
        to: Version,
        lossy: bool,
        note: &'static str,
        transform: fn(&Value) -> Result<Value, LensError>,
    ) -> Self {
        Self {
            product_group: product_group.into(),
            from,
            to,
            lossy,
            note,
            transform,
        }
    }
}

/// A lens transform failed on its input. A well-formed lens over data that
/// validates against `from` never returns this; it exists so a transform can
/// refuse structurally impossible input rather than silently corrupt it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LensError(pub String);

impl std::fmt::Display for LensError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "lens transform failed: {}", self.0)
    }
}
