//! Core DPP domain types: passport, GTIN, identity, status, product group, validation,
//! and transfer of responsibility.

pub mod compliance;
pub mod eol;
pub mod identifier;

pub mod graph;
#[cfg(test)]
mod graph_tests;

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
