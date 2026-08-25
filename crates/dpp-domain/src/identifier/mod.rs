//! Product and party identifiers — the vocabulary tier.
//!
//! Every type here names a thing and decides nothing, and imports only `serde`
//! and `thiserror`. That is what makes this the one part of the crate a
//! `wasm32` consumer could take on its own: `dpp-registry` uses exactly three
//! items from all of `dpp-domain`, and all three are here.
//!
//! The two CN types sit side by side deliberately. [`CommodityCode`] is a
//! product's own 6/8/10-digit classification; [`CnCategory`] is the CN chapter
//! (2 digits) or heading (4) that ESPR Art. 3 delimits a disclosure category by.
//! They look alike and are not interchangeable, which is easier to see when they
//! are neighbours.

mod check_digit;
pub mod cn_category;
pub mod commodity_code;
pub mod gln;
pub mod gtin;
#[cfg(test)]
mod tests;

pub use check_digit::gs1_check_digit;
pub use cn_category::{CnCategory, CnCategoryError};
pub use commodity_code::{CommodityCode, CommodityCodeError};
pub use gln::{Gln, GlnError};
pub use gtin::{Gtin, GtinError};
