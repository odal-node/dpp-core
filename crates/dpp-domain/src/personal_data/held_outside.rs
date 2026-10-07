//! [`HeldOutside`] — where personal data related to a marked field is kept, and
//! on what basis.

use serde::{Deserialize, Serialize};

use super::lawful_basis::LawfulBasis;
use super::record_id::PersonalDataRecordId;

/// Personal data related to a marked field, held in an erasable record outside
/// the passport.
///
/// Carries the basis and the record's identifier, and nothing about the person.
/// The controller's own evidence of the basis, such as its record of a consent,
/// is deliberately not referenced here: the controller keeps that evidence after
/// the data is erased, so a pointer to it in a signed passport would keep
/// relating the passport to the person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct HeldOutside {
    /// The GDPR Art. 6(1) ground the record is held on.
    pub lawful_basis: LawfulBasis,
    /// The record holding the data.
    pub record: PersonalDataRecordId,
}

impl HeldOutside {
    /// A record held on `lawful_basis`.
    #[must_use]
    pub fn new(lawful_basis: LawfulBasis, record: PersonalDataRecordId) -> Self {
        Self {
            lawful_basis,
            record,
        }
    }
}
