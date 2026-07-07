//! Minimal JSON-LD context builder for DPP passport payloads.
//!
//! Intentionally small and flat: three cohesive functions (build/frame/strip
//! the `@context` envelope), not a growing vocabulary layer. If this module
//! starts accumulating per-vocabulary mapping logic, split it then — until
//! then, one file is the right size.

mod context;
#[cfg(test)]
mod tests;

pub use context::{frame_passport, passport_context, strip_context};
