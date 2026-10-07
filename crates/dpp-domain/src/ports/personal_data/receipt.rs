//! [`ErasureReceipt`] — what is left of a record once it is erased.

use chrono::{DateTime, Utc};

use crate::passport::PassportId;
use crate::personal_data::PersonalDataRecordId;

/// The tombstone of an erased record: which record it was, which passport field
/// it related to, and when it was erased. No content, and nothing about the
/// person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErasureReceipt {
    /// The erased record's identifier.
    pub record: PersonalDataRecordId,
    /// The passport it related to.
    pub passport_id: PassportId,
    /// The marked field it related to.
    pub field: String,
    /// When it was erased.
    pub erased_at: DateTime<Utc>,
}
