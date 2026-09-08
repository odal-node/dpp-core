//! [`PropertyChange`] and [`ChangeKind`] — one property's movement between two
//! schema versions, and how to classify it.
//!
//! Split from [`diff`](super::diff) rather than living beside [`SchemaDiff`](super::SchemaDiff)
//! because these two describe a single property while that describes a whole
//! version bump. They are also the pair a caller matches on, so they are worth
//! finding under their own name.

/// The keywords treated as constraints on a property's value.
///
/// A change to any of these narrows or widens what an operator may declare,
/// which is the kind of change a reader needs to see. Sorted so the rendered
/// report is stable regardless of the order they appear in the schema.
pub(super) const CONSTRAINT_KEYWORDS: &[&str] = &[
    "enum",
    "exclusiveMaximum",
    "exclusiveMinimum",
    "format",
    "maxItems",
    "maxLength",
    "maximum",
    "minItems",
    "minLength",
    "minimum",
    "multipleOf",
    "pattern",
    "uniqueItems",
];

/// One property whose declaration changed between two versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyChange {
    /// Path from the schema root, `/`-separated, with `[]` for an array's
    /// element schema — e.g. `materials/[]/weightKg`.
    pub path: String,
    /// What changed about it.
    pub kind: ChangeKind,
    /// The previous value, rendered. Empty for an addition, and empty for a
    /// property that declares no `type` at all.
    pub before: String,
    /// The new value, rendered. Empty for a removal, and for the same
    /// no-declared-type case.
    pub after: String,
}

/// How a property changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// The property did not exist in the previous version.
    Added,
    /// The property existed and no longer does.
    Removed,
    /// The property's `type` changed.
    TypeChanged,
    /// A constraint keyword — `pattern`, `enum`, `minimum`, `format` and the
    /// rest of the value-constraining set — was added, removed or altered.
    ConstraintChanged,
}

impl ChangeKind {
    /// Stable label for rendering.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Removed => "removed",
            Self::TypeChanged => "type changed",
            Self::ConstraintChanged => "constraint changed",
        }
    }
}
