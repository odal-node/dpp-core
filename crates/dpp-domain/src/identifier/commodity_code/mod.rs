//! [`CommodityCode`] — a product's own 6/8/10-digit CN classification.

mod code;
mod declared;
mod error;
#[cfg(test)]
mod tests;

pub use code::CommodityCode;
pub use declared::DeclaredCommodityCode;
pub use error::CommodityCodeError;
