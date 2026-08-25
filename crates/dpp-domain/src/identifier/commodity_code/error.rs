//! [`CommodityCodeError`] — why a commodity code was refused.

use thiserror::Error;

/// Error from constructing a [`CommodityCode`](super::CommodityCode).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CommodityCodeError {
    /// Not 6, 8 or 10 ASCII digits.
    #[error("commodity code must be 6 (HS), 8 (CN) or 10 (TARIC) ASCII digits, got '{0}'")]
    InvalidFormat(String),
}
