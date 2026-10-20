//! [`GhostRegistrySync`] — a no-op registry sync for development.

use async_trait::async_trait;
use uuid::Uuid;

use crate::error::dpp::DppError;
use crate::ports::registry_sync::{
    RegistrationRequest, RegistryRecord, RegistrySyncPort, SubmissionAcknowledgement,
    SubmissionLookup, SubmissionReport, TransferReport,
};

/// No-op implementation for use before a deployment is enrolled with the EU
/// Central Registry.
///
/// Accepts every submission under a placeholder correlation identifier and
/// knows of no submission or record afterwards: nothing was sent, so there is
/// nothing to report on, and answering `Processing` would keep a caller polling
/// a registry that does not exist.
pub struct GhostRegistrySync;

#[async_trait]
impl RegistrySyncPort for GhostRegistrySync {
    async fn register(
        &self,
        _request: RegistrationRequest,
    ) -> Result<SubmissionAcknowledgement, DppError> {
        Ok(SubmissionAcknowledgement::Accepted {
            correlation_id: format!("GHOST-CORR-{}", Uuid::now_v7()),
        })
    }

    async fn check_submission(&self, correlation_id: &str) -> Result<SubmissionReport, DppError> {
        Err(DppError::NotFound(format!(
            "ghost registry has no submission {correlation_id}"
        )))
    }

    /// Nothing was ever submitted, so there is nothing to find, and all of it was
    /// read.
    async fn find_submission(
        &self,
        _request: &RegistrationRequest,
    ) -> Result<SubmissionLookup, DppError> {
        Ok(SubmissionLookup::NotFound { exhaustive: true })
    }

    async fn check_status(&self, registration_id: &str) -> Result<RegistryRecord, DppError> {
        Err(DppError::NotFound(format!(
            "ghost registry has no record {registration_id}"
        )))
    }

    async fn notify_transfer(
        &self,
        record: &crate::transfer::TransferRecord,
        _registration_id: &str,
    ) -> Result<TransferReport, DppError> {
        Err(DppError::NotFound(format!(
            "ghost registry has no record for {}",
            record.passport_id
        )))
    }
}
