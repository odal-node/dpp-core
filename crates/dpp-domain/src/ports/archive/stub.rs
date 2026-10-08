//! [`InMemoryArchive`] — a `HashMap`-backed archive for tests and local runs.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::{ArchiveReceipt, ArchivedVersion, ArchivedVersionPort};
use crate::error::DppError;
use crate::field_error::ValidationErrors;
use crate::passport::PassportId;

/// A version with the receipt it was given, kept so that a retry can be answered
/// with the original rather than a fresh one.
struct Held {
    version: ArchivedVersion,
    receipt: ArchiveReceipt,
}

/// Holds every archived version in memory, oldest first, and honours the whole
/// contract of [`ArchivedVersionPort`]: ordering, retry and refusal included.
pub struct InMemoryArchive {
    held: Mutex<HashMap<PassportId, Vec<Held>>>,
}

impl InMemoryArchive {
    pub fn new() -> Self {
        Self {
            held: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryArchive {
    fn default() -> Self {
        Self::new()
    }
}

fn refused(reason: String) -> DppError {
    DppError::Validation(ValidationErrors::message(reason))
}

#[async_trait]
impl ArchivedVersionPort for InMemoryArchive {
    async fn archive(
        &self,
        passport_id: PassportId,
        doc: &serde_json::Value,
        superseded_at: DateTime<Utc>,
    ) -> Result<ArchiveReceipt, DppError> {
        let content_hash = dpp_rules::canonical::content_hash(doc)
            .map_err(|e| DppError::Serialisation(e.to_string()))?;
        let mut held = self.held.lock().unwrap();
        let series = held.entry(passport_id).or_default();

        if let Some(same_instant) = series
            .iter()
            .find(|h| h.version.superseded_at == superseded_at)
        {
            return if same_instant.version.content_hash == content_hash {
                Ok(same_instant.receipt.clone())
            } else {
                Err(refused(format!(
                    "a different version of {passport_id} is already archived as superseded at \
                     {superseded_at}"
                )))
            };
        }
        if let Some(latest) = series.last()
            && latest.version.superseded_at > superseded_at
        {
            return Err(refused(format!(
                "a version of {passport_id} superseded at {superseded_at} would precede the \
                 latest archived one, superseded at {}",
                latest.version.superseded_at
            )));
        }

        let receipt = ArchiveReceipt {
            passport_id,
            superseded_at,
            content_hash: content_hash.clone(),
            archived_at: Utc::now(),
        };
        series.push(Held {
            version: ArchivedVersion {
                passport_id,
                doc: doc.clone(),
                superseded_at,
                content_hash,
            },
            receipt: receipt.clone(),
        });
        Ok(receipt)
    }

    async fn versions(&self, passport_id: PassportId) -> Result<Vec<ArchivedVersion>, DppError> {
        let held = self.held.lock().unwrap();
        Ok(held
            .get(&passport_id)
            .map(|series| series.iter().map(|h| h.version.clone()).collect())
            .unwrap_or_default())
    }

    async fn version_at(
        &self,
        passport_id: PassportId,
        at: DateTime<Utc>,
    ) -> Result<Option<ArchivedVersion>, DppError> {
        let held = self.held.lock().unwrap();
        Ok(held.get(&passport_id).and_then(|series| {
            series
                .iter()
                .find(|h| h.version.superseded_at > at)
                .map(|h| h.version.clone())
        }))
    }
}
