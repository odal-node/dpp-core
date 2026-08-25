//! Core DPP domain types: passport, GTIN, identity, status, product group, validation,
//! and transfer of responsibility.

pub mod commodity_code;
#[cfg(test)]
mod commodity_code_tests;
pub mod compliance;
pub mod eol;
pub mod error;
#[cfg(test)]
mod error_tests;
pub mod field_error;
pub mod graph;
#[cfg(test)]
mod graph_tests;
pub mod gtin;
pub mod identity;
pub mod lint;
pub mod passport;
#[cfg(test)]
mod passport_status_all_tests;
pub mod product_group;
pub mod product_identity;
#[cfg(test)]
mod product_identity_tests;
pub mod seal;
pub mod status;
#[cfg(test)]
mod status_tests;
pub mod transfer;
#[cfg(not(target_arch = "wasm32"))]
pub mod validation;
