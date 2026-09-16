//! Electronics — EU Ecodesign Regulation (ESPR), product group rules.
//! NOTE: repairability scoring (EN 45554 A–E grades, weighted) lives in dpp-calc, not here.
pub mod repairability_index_scope;
#[cfg(test)]
mod repairability_index_scope_tests;
pub mod spare_parts;
