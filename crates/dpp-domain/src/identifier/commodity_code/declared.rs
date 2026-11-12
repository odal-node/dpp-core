//! [`DeclaredCommodityCode`] — the classification a document says, valid or not.

use serde::{Deserialize, Serialize};

use super::code::CommodityCode;
use super::error::CommodityCodeError;

/// The customs tariff classification a stored or fetched document declares, kept
/// exactly as written.
///
/// # Why this is not [`CommodityCode`]
///
/// [`CommodityCode`] is valid by construction, and reading one refuses what
/// [`CommodityCode::parse`] refuses. A document that is already written is a
/// different case. A stored passport read again is read after the fact, and a
/// fetched one is signed by another operator and can never be rewritten on our
/// side, so a code that was accepted when the document was made has to go on
/// reading, and to write back as the very string it was. This type is that: any
/// string, carried verbatim, with the question of whether it is a valid code
/// asked separately by [`validated`](Self::validated).
///
/// So a malformed code is not hidden and not refused on read. It is reported where
/// it is used: the registry refuses to register one, and the discard-record lint
/// flags one.
///
/// To build one from input that must be valid, [`CommodityCode::parse`] it first;
/// [`From<CommodityCode>`](CommodityCode) wraps the result.
///
/// On the wire it is the bare string, which is what `CommodityCode` also writes,
/// so a document is the same JSON whichever of the two it was written from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeclaredCommodityCode(String);

impl DeclaredCommodityCode {
    /// Keep `code` as written. Nothing is checked.
    #[must_use]
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into())
    }

    /// The code exactly as the document wrote it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The code as a [`CommodityCode`], or why it is not one.
    ///
    /// # Errors
    ///
    /// [`CommodityCodeError::InvalidFormat`] where [`CommodityCode::parse`] refuses
    /// it: not 6, 8 or 10 digits.
    pub fn validated(&self) -> Result<CommodityCode, CommodityCodeError> {
        CommodityCode::parse(&self.0)
    }
}

impl From<CommodityCode> for DeclaredCommodityCode {
    /// Declare a code that is already known to be valid.
    fn from(code: CommodityCode) -> Self {
        Self(code.as_str().to_owned())
    }
}

impl std::fmt::Display for DeclaredCommodityCode {
    /// The code as written.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
