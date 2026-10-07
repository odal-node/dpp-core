//! [`InMemoryPersonalData`] — a `HashMap`-backed holder for tests and local
//! runs.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;

use super::{ErasureReceipt, HeldRecord, PersonalDataPort, PersonalDataRecord};
use crate::error::DppError;
use crate::passport::PassportId;
use crate::personal_data::PersonalDataRecordId;

/// Holds every record in memory and honours the whole contract of
/// [`PersonalDataPort`]: erasure that keeps only a tombstone, and a retry that
/// returns the first receipt.
pub struct InMemoryPersonalData {
    held: Mutex<HashMap<PersonalDataRecordId, HeldRecord>>,
}

impl InMemoryPersonalData {
    pub fn new() -> Self {
        Self {
            held: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryPersonalData {
    fn default() -> Self {
        Self::new()
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
        let record = PersonalDataRecord {
            id: id.clone(),
            passport_id,
            field: field.to_owned(),
            content: content.clone(),
            stored_at: Utc::now(),
        };
        self.held
            .lock()
            .unwrap()
            .insert(id.clone(), HeldRecord::Present(record));
        Ok(id)
    }

    async fn fetch(&self, record: &PersonalDataRecordId) -> Result<Option<HeldRecord>, DppError> {
        Ok(self.held.lock().unwrap().get(record).cloned())
    }

    async fn erase(&self, record: &PersonalDataRecordId) -> Result<ErasureReceipt, DppError> {
        let mut held = self.held.lock().unwrap();
        let Some(entry) = held.get_mut(record) else {
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
