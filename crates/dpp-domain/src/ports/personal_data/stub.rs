//! [`InMemoryPersonalData`] — a `Vec`-backed holder for tests and local runs.

use std::sync::{Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use chrono::Utc;

use super::{ErasureReceipt, HeldRecord, PersonalDataPort, PersonalDataRecord};
use crate::error::DppError;
use crate::passport::PassportId;
use crate::personal_data::PersonalDataRecordId;

/// Holds every record in memory, in the order stored, and honours the whole
/// contract of [`PersonalDataPort`]: erasure that keeps only a tombstone, a
/// retried erasure that returns the first receipt, and a listing per passport.
pub struct InMemoryPersonalData {
    pub(super) held: Mutex<Vec<HeldRecord>>,
}

impl InMemoryPersonalData {
    pub fn new() -> Self {
        Self {
            held: Mutex::new(Vec::new()),
        }
    }

    /// The records, whatever happened to an earlier holder of the lock. Every
    /// write here replaces one whole entry, so a panic elsewhere cannot leave
    /// one half-written, and refusing to answer would leave a record that
    /// could no longer be erased.
    fn held(&self) -> MutexGuard<'_, Vec<HeldRecord>> {
        self.held.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for InMemoryPersonalData {
    fn default() -> Self {
        Self::new()
    }
}

fn id_of(held: &HeldRecord) -> &PersonalDataRecordId {
    match held {
        HeldRecord::Present(record) => &record.id,
        HeldRecord::Erased(receipt) => &receipt.record,
    }
}

fn passport_of(held: &HeldRecord) -> PassportId {
    match held {
        HeldRecord::Present(record) => record.passport_id,
        HeldRecord::Erased(receipt) => receipt.passport_id,
    }
}

#[async_trait]
impl PersonalDataPort for InMemoryPersonalData {
    async fn store(
        &self,
        passport_id: PassportId,
        field: &str,
        content: &serde_json::Value,
    ) -> Result<PersonalDataRecordId, DppError> {
        let id = PersonalDataRecordId::new(uuid::Uuid::now_v7().to_string());
        self.held().push(HeldRecord::Present(PersonalDataRecord {
            id: id.clone(),
            passport_id,
            field: field.to_owned(),
            content: content.clone(),
            stored_at: Utc::now(),
        }));
        Ok(id)
    }

    async fn fetch(&self, record: &PersonalDataRecordId) -> Result<Option<HeldRecord>, DppError> {
        Ok(self.held().iter().find(|h| id_of(h) == record).cloned())
    }

    async fn records_for(&self, passport_id: PassportId) -> Result<Vec<HeldRecord>, DppError> {
        Ok(self
            .held()
            .iter()
            .filter(|h| passport_of(h) == passport_id)
            .cloned()
            .collect())
    }

    async fn erase(&self, record: &PersonalDataRecordId) -> Result<ErasureReceipt, DppError> {
        let mut held = self.held();
        let Some(entry) = held.iter_mut().find(|h| id_of(h) == record) else {
            return Err(DppError::NotFound(format!(
                "no personal data record {record} is held here"
            )));
        };
        let receipt = match entry {
            HeldRecord::Erased(receipt) => return Ok(receipt.clone()),
            HeldRecord::Present(present) => ErasureReceipt {
                record: present.id.clone(),
                passport_id: present.passport_id,
                field: present.field.clone(),
                erased_at: Utc::now(),
            },
        };
        *entry = HeldRecord::Erased(receipt.clone());
        Ok(receipt)
    }
}
