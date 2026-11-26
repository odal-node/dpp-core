//! Cross-product group helper rules: country code validation, unique product
//! identifier syntax, numeric utilities, unit conversions, and the
//! dependency-free date key used for regulatory phase selection.

pub mod country;
pub mod date;
pub mod identifier;
pub mod numeric;
#[cfg(test)]
mod tests;
pub mod units;
