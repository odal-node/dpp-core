//! [`CnCategoryError`] — why a CN category was refused.

use thiserror::Error;

/// Error from constructing a [`CnCategory`](super::CnCategory).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CnCategoryError {
    /// Not 2 or 4 ASCII digits.
    #[error("CN category must be 2 (chapter) or 4 (heading) ASCII digits, got '{0}'")]
    InvalidFormat(String),
}
