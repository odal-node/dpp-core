//! Core DPP domain types: passport, GTIN, identity, status, product group, validation,
//! and transfer of responsibility.

pub mod commodity_code;
pub mod compliance;
pub mod eol;
pub mod error;
pub mod field_error;
pub mod graph;
pub mod gtin;
pub mod identity;
pub mod lint;
pub mod passport;
pub mod product_group;
pub mod product_identity;
pub mod seal;
pub mod status;
pub mod transfer;
#[cfg(not(target_arch = "wasm32"))]
pub mod validation;
