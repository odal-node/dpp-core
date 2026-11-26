//! Unsold-goods destruction ban — ESPR Art. 25 / Annex VII scope.
//!
//! ## Annex VII scope test (cross-field)
//! Annex VII, "Consumer products of which the destruction by economic
//! operators is prohibited", is a commodity-code table with exactly two
//! headings, taken from the combined nomenclature (Council Regulation (EEC)
//! No 2658/87, Annex I, version in force 28 June 2024):
//!
//! - Apparel and clothing accessories: CN `4203`, `61`, `62`, `6504`, `6505`
//! - Footwear: CN `6401`–`6405`
//!
//! The codes are chapter- (2-digit) and heading-level (4-digit); a product's
//! own commodity code is 6, 8 or 10 digits, so the scope test is **prefix
//! matching**, never equality.

mod heading;
#[cfg(test)]
mod tests;

pub use heading::*;
