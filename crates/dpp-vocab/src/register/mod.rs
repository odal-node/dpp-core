//! The set of recorded vocabularies, and the gate over it.
//!
//! [`crate::vocabulary`] holds the shape of one record; this module holds all of
//! them, because the question *"may this IRI be emitted?"* needs the whole set
//! rather than any single record.
//!
//! ## Module layout
//!
//! Submodules are private and their types re-exported, so each type has exactly
//! one public path.
//!
//! - `register` — [`VocabularyRegister`] itself, plus the embedded records.
//! - `verdict` — [`Verdict`], the answer the gate gives and the reason with it.

#[allow(clippy::module_inception)]
mod register;
#[cfg(test)]
mod tests;
mod verdict;

pub use register::VocabularyRegister;
pub use verdict::Verdict;
