//! The economic operator answerable for a product, and the law that makes it so.
//!
//! A value object with no aggregate of its own, which is why it sits here rather
//! than inside `transfer`: the transfer chain records who has been responsible,
//! and the passport states who is responsible, so neither owns the type. Both
//! import it.
//!
//! ## Module layout
//!
//! - [`responsible`] — [`ResponsibleOperator`] and its [`OperatorRole`].
//! - [`basis`] — [`ResponsibilityBasis`], which law makes an operator answerable.
//! - [`snapshot`] — [`ResponsibleOperatorSnapshot`], the pair as the passport carries it.

pub mod basis;
pub mod responsible;
pub mod snapshot;

#[cfg(test)]
mod basis_tests;
#[cfg(test)]
mod responsible_tests;
#[cfg(test)]
mod snapshot_tests;

pub use basis::ResponsibilityBasis;
pub use responsible::{OperatorRole, ResponsibleOperator};
pub use snapshot::ResponsibleOperatorSnapshot;
