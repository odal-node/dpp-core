//! [`HeldSummary`]: what a listing says about a held record, without its data.

use chrono::{DateTime, Utc};

use super::held::HeldRecord;
use super::receipt::ErasureReceipt;
use crate::passport::PassportId;
use crate::personal_data::PersonalDataRecordId;

/// What a listing says about one held record: its identifier, passport, field
/// and when it was stored, or its tombstone if it was erased. None of the
/// content.
///
/// The conversion from a [`HeldRecord`] names each field it carries over and
/// never the content, so data added to a record later cannot ride along into a
/// listing.
#[derive(Debug, Clone, PartialEq)]
pub enum HeldSummary {
    /// A record that is still held.
    Present {
        /// The record's identifier.
        record: PersonalDataRecordId,
        /// The passport it relates to.
        passport_id: PassportId,
        /// The marked field it relates to.
        field: String,
        /// When it was stored.
        stored_at: DateTime<Utc>,
    },
    /// A record that was erased, and its tombstone.
    Erased(ErasureReceipt),
}

impl From<&HeldRecord> for HeldSummary {
    fn from(held: &HeldRecord) -> Self {
        match held {
            HeldRecord::Present(record) => Self::Present {
                record: record.id.clone(),
                passport_id: record.passport_id,
                field: record.field.clone(),
                stored_at: record.stored_at,
            },
            HeldRecord::Erased(receipt) => Self::Erased(receipt.clone()),
        }
    }
}
