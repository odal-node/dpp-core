//! The crate-wide error surface.
//!
//! This module sits **above** the tier ladder in `CODE-LAYOUT.md` §1 rather than
//! inside it: a crate-wide error has to be able to name a type from any tier, so
//! constraining which tiers it may reach would only push the coupling somewhere
//! less visible.
//!
//! - [`dpp`] — [`DppError`], the one error every fallible entry point returns.
//! - [`field`] — [`FieldError`] and [`ValidationErrors`], the per-field detail a
//!   validation failure carries.

pub mod dpp;
pub mod field;
#[cfg(test)]
mod tests;

pub use dpp::DppError;
pub use field::{FieldError, ValidationErrors};
