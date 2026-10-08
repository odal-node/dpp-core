//! [`HeldRecord`] — what a lookup finds under a record identifier.

use super::receipt::ErasureReceipt;
use super::record::PersonalDataRecord;

/// What is held under a record identifier this port minted.
#[derive(Debug, Clone, PartialEq)]
pub enum HeldRecord {
    /// The record, with its data.
    Present(PersonalDataRecord),
    /// The record was erased, and this is its tombstone.
    Erased(ErasureReceipt),
}
