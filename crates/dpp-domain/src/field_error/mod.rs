//! Per-field validation detail — [`FieldError`] and its [`ValidationErrors`]
//! collection.
//!
//! Tier 2, separate from [`crate::error`], and the split is not cosmetic. These
//! are value types the schema and validation tiers *produce*; `DppError` is the
//! crate-wide error that *wraps* them, and it also wraps a tier-3 lens error.
//! Holding both in one module made `error` simultaneously below `schemas` and
//! above it, which is a cycle — and one the tier gate could not see, because it
//! checked direction and not cycles.

mod error;

pub use error::{FieldError, ValidationErrors};
