//! The legal instruments the passport regime is built on — the acts themselves,
//! how they bind a product group, and what obligation they create.
//!
//! Lifted out of `catalog/` because it is a different axis. The product-group
//! catalog says what a product group *is*: identity, scope, schema versions,
//! plugin binding. It carries no law. The instrument catalog says which acts
//! reach a product group and what they require. Filing both under one name made
//! nine files share a directory and a reader share two subjects.

pub mod act;
pub mod binding;
pub mod catalog;
pub mod kind;
#[cfg(test)]
mod kind_tests;
pub mod obligation;
pub mod reference;
#[cfg(test)]
mod reference_tests;
pub mod status;
#[cfg(test)]
mod tests;

pub use act::Instrument;
pub use binding::InstrumentBinding;
pub use catalog::InstrumentCatalog;
pub use kind::InstrumentKind;
pub use obligation::{DateBasis, ObligationDate, PassportObligation};
pub use reference::{InstrumentRef, RecordedBasis};
pub use status::InstrumentStatus;
