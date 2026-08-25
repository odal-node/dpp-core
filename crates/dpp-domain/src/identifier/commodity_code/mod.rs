//! [`CommodityCode`] — a product's own 6/8/10-digit CN classification.

mod code;
mod error;
#[cfg(test)]
mod tests;

pub use code::CommodityCode;
pub use error::CommodityCodeError;
