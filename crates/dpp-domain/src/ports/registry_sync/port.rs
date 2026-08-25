//! [`RegistrySyncPort`] — the contract an EU Central Registry adapter implements.

use async_trait::async_trait;

use super::record::RegistryRecord;
use super::request::RegistrationRequest;
use crate::domain::{error::DppError, passport::PassportId};

/// Port trait for synchronising DPP records with the EU Central Registry.
///
/// The Commission's registry API specification is pending (expected mid-2026).
/// This trait defines the contract that platform adapters will implement.
///
/// # Ghost implementation
///
/// Until the API is published, platform code should wire `GhostRegistrySync`
/// which logs the call and returns a synthetic `RegistryRecord` with
/// `RegistryStatus::Pending`.
#[async_trait]
pub trait RegistrySyncPort: Send + Sync {
    /// Register a new DPP with the EU Central Registry.
    ///
    /// Called when a passport transitions from Draft to Published.
    /// Returns the registry's confirmation record with assigned identifiers.
    async fn register(&self, request: RegistrationRequest) -> Result<RegistryRecord, DppError>;

    /// Query the current status of a previously registered DPP.
    async fn check_status(&self, passport_id: PassportId) -> Result<RegistryRecord, DppError>;

    /// Update a registry record after a transfer of responsibility.
    ///
    /// Called when a product's responsible economic operator changes
    /// (e.g. remanufacturing, repurposing, import into a new market).
    ///
    /// `registry_id` is the registry's own record identifier for this passport,
    /// returned when it was registered. Without it the registry has no way to
    /// know which record the handover refers to, so a caller that does not yet
    /// have one must wait rather than send an unattached notification.
    ///
    /// Takes the whole [`TransferRecord`](crate::domain::transfer::TransferRecord)
    /// rather than just the incoming
    /// operator's identifier. A registry notification names **both** legal
    /// persons and carries the dual signatures that authorise the handover;
    /// passing only the new identifier left an adapter no way to express the
    /// outgoing operator or either signature, so it could only send empty
    /// strings for data the system had already collected.
    async fn notify_transfer(
        &self,
        record: &crate::domain::transfer::TransferRecord,
        registry_id: &str,
    ) -> Result<RegistryRecord, DppError>;
}
