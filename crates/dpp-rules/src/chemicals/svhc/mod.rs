//! SVHC concentration validation — REACH Art. 33 / ECHA SCIP.
//!
//! Cross-product group: applies to textiles, electronics, toys, construction, and more.
//! Kept here rather than under any single product group for that reason.

mod candidate_list;
#[cfg(test)]
mod tests;

pub use candidate_list::*;
