//! [`InMemoryArchive`] — a `HashMap`-backed archive for tests and local runs.

use super::*;
use crate::error::DppError;
use crate::passport::{Passport, PassportId};
use async_trait::async_trait;
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;

pub struct InMemoryArchive {
    store: Mutex<HashMap<PassportId, (Passport, ArchiveReceipt)>>,
}

impl InMemoryArchive {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }

    fn hash_passport(passport: &Passport) -> String {
        let json = serde_json::to_vec(passport).unwrap_or_default();
        let digest = Sha256::digest(&json);
        hex::encode(digest)
    }
}

impl Default for InMemoryArchive {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ArchivePort for InMemoryArchive {
    async fn archive(
        &self,
        passport: &Passport,
        retention_years: u32,
    ) -> Result<ArchiveReceipt, DppError> {
        let now = Utc::now();
        let retention_until = retention_deadline(now, retention_years);
        let hash = Self::hash_passport(passport);
        let receipt = ArchiveReceipt {
            archive_id: format!("ARCHIVE-{}", uuid::Uuid::now_v7()),
            passport_id: passport.id,
            content_hash: hash,
            archived_at: now,
            retention_until,
        };
        let mut store = self.store.lock().unwrap();
        store.insert(passport.id, (passport.clone(), receipt.clone()));
        Ok(receipt)
    }

    async fn update_archive(&self, passport: &Passport) -> Result<ArchiveReceipt, DppError> {
        let mut store = self.store.lock().unwrap();
        if let Some((stored, receipt)) = store.get_mut(&passport.id) {
            *stored = passport.clone();
            receipt.content_hash = Self::hash_passport(passport);
            Ok(receipt.clone())
        } else {
            Err(DppError::NotFound(format!(
                "no archived record for {}",
                passport.id
            )))
        }
    }

    async fn verify(
        &self,
        passport_id: PassportId,
        expected_hash: &str,
    ) -> Result<ArchiveVerification, DppError> {
        let store = self.store.lock().unwrap();
        if let Some((_, receipt)) = store.get(&passport_id) {
            Ok(ArchiveVerification {
                integrity_ok: receipt.content_hash == expected_hash,
                accessible: true,
                status: ArchiveStatus::Active,
                last_verified_at: Utc::now(),
            })
        } else {
            Err(DppError::NotFound(format!(
                "no archived record for {passport_id}"
            )))
        }
    }

    async fn retrieve(&self, passport_id: PassportId) -> Result<Option<Passport>, DppError> {
        let store = self.store.lock().unwrap();
        Ok(store.get(&passport_id).map(|(p, _)| p.clone()))
    }
}
