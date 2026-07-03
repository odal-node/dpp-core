//! Core DPP domain types: passport, GTIN, identity, status, sector, validation,
//! and transfer of responsibility.

pub mod eol;
pub mod error;
pub mod field_error;
pub mod gtin;
pub mod identity;
pub mod passport;
pub mod sector;
pub mod status;
pub mod transfer;
#[cfg(not(target_arch = "wasm32"))]
pub mod validation;
