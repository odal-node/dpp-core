//! Shared fixture builders for the integration tests in `tests/` (and the
//! `dpp-benches` crate, which depends on this library as a dev-dependency).
//!
//! These are the product group-agnostic envelope / actor shapes that were hand-rolled
//! at multiple call sites before this module existed.

mod builders;

pub use builders::*;
