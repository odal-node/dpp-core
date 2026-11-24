//! [`ErasureReceipt`] — what is left of a record once it is erased.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::passport::PassportId;
use crate::personal_data::PersonalDataRecordId;

/// The tombstone of an erased record: which record it was, which passport field
/// it related to, and when it was erased. None of the erased content. The
/// passport identifier can still relate to whoever owns the item, so see the
/// module docs on how long a tombstone is kept.
///
/// Serialisable because a holder answers an erasure with it, and a receipt that
/// every holder re-declares in order to put it on a wire is a receipt that can be
/// spelt two ways. The keys are camelCase, as every other record here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
