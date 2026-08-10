//! Our own identifier namespace, and the rule that gives it its meaning.
//!
//! ## Module layout
//!
//! Submodules are private and their items re-exported, so each has exactly one
//! public path.
//!
//! - `own` — [`OWN_NAMESPACE`] and [`is_own`].

mod own;

pub use own::{OWN_NAMESPACE, is_own};
