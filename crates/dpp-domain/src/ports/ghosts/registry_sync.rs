//! [`GhostRegistrySync`] — a no-op registry sync for development.

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::error::dpp::DppError;
use crate::passport::PassportId;
use crate::ports::registry_sync::{
    RegistrationRequest, RegistryIdentifiers, RegistryRecord, RegistryStatus, RegistrySyncPort,
};

/// No-op implementation for use before the EU Central Registry API is published.
///
/// Returns synthetic records with `RegistryStatus::Pending` and placeholder
/// identifiers. All operations succeed but perform no real network calls.
pub struct GhostRegistrySync;

#[async_trait]
impl RegistrySyncPort for GhostRegistrySync {
    async fn register(&self, request: RegistrationRequest) -> Result<RegistryRecord, DppError> {
        let now = Utc::now();
        Ok(RegistryRecord {
            identifiers: RegistryIdentifiers {
                product_id: format!("GHOST-PROD-{}", request.passport_id),
                operator_id: format!("GHOST-OP-{}", &request.operator_identifier),
                facility_id: format!("GHOST-FAC-{}", &request.facility_identifier),
                registry_id: format!("GHOST-REG-{}", Uuid::now_v7()),
            },
            status: RegistryStatus::Pending,
            registered_at: now,
            updated_at: now,
        })
    }

    async fn check_status(&self, passport_id: PassportId) -> Result<RegistryRecord, DppError> {
        Err(DppError::NotFound(format!(
            "ghost registry has no record for {passport_id}"
        )))
    }

    async fn notify_transfer(
        &self,
        record: &crate::transfer::TransferRecord,
        _registry_id: &str,
    ) -> Result<RegistryRecord, DppError> {
        Err(DppError::NotFound(format!(
            "ghost registry has no record for {}",
            record.passport_id
        )))
    }
}
