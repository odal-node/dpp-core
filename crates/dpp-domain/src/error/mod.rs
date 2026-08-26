//! The crate-wide error surface.
//!
//! [`DppError`] is the one error every fallible entry point returns. It sits at
//! the level of the deepest thing it wraps — a lens error from
//! [`crate::schemas`] — which is why it is *not* the same module as
//! [`crate::field_error`], the per-field detail it carries. Holding both here
//! made this module simultaneously above and below `schemas`, and that cycle
//! was invisible to a check that only looked at direction.

pub mod dpp;
#[cfg(test)]
mod tests;

pub use dpp::DppError;
