//! [`PersonalDataRecordId`] — names a record of personal data held outside a
//! passport.

use serde::{Deserialize, Serialize};

/// Identifies a record of personal data held outside a passport, through the
/// `PersonalDataPort` in the ports module.
///
/// Opaque, and minted by whoever holds the record. It goes into a signed,
/// retention-locked passport, where it outlives the record it names, so it must
/// carry nothing derived from the data or from the person: once the record is
/// erased, the identifier left in the passport must say nothing about anyone.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PersonalDataRecordId(String);

impl PersonalDataRecordId {
    /// Wrap an identifier minted by the record's holder.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The identifier as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PersonalDataRecordId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
