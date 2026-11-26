//! Test-only synthetic [`FactorProvider`] returning hardcoded factors.
//!
//! The whole module is compiled only under `cfg(test)` or the
//! `synthetic-factors` feature, so it can never ship in a binary that makes
//! compliance claims.

mod provider;
#[cfg(test)]
mod tests;

pub use provider::*;
