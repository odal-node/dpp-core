//! The unsold-goods disclosure as an AAS Environment, for a file export.
//!
//! - [`environment`] — [`build_unsold_goods_environment`], the entry point.
//! - [`submodel`] — the disclosure's submodel.

mod environment;
mod submodel;

pub use environment::build_unsold_goods_environment;
