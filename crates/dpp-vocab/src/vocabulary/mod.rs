//! What an upstream vocabulary *is* — one record per authority.
//!
//! This module holds the shape only. The set of records lives in
//! [`crate::register`], and the question of whether a given IRI may be emitted
//! is answered there, because it needs the whole set.
//!
//! ## Module layout
//!
//! Submodules are private and their types re-exported, so each type has exactly
//! one public path.
//!
//! - `layer` — [`Layer`], what a vocabulary is *for* and therefore how it may
//!   be used.
//! - `status` — [`Status`], how much of a record rests on somebody having read
//!   the authority.
//! - `record` — [`Vocabulary`], the record shape itself.

mod layer;
mod record;
mod status;

pub use layer::Layer;
pub use record::Vocabulary;
pub use status::Status;
