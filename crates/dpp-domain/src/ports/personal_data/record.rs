//! [`PersonalDataRecord`] — personal data related to one marked field of one
//! passport.

use chrono::{DateTime, Utc};

use crate::passport::PassportId;
use crate::personal_data::PersonalDataRecordId;

/// Personal data related to one marked field of one passport, as held.
#[derive(Debug, Clone, PartialEq)]
pub struct PersonalDataRecord {
    /// The record's identifier, as the passport's statement names it.
    pub id: PersonalDataRecordId,
    /// The passport the record relates to.
    pub passport_id: PassportId,
    /// The marked field, as a dotted path inside `productGroupData`.
    pub field: String,
    /// The data, in whatever shape the controller keeps it.
    pub content: serde_json::Value,
    /// When the record was stored.
    pub stored_at: DateTime<Utc>,
}
