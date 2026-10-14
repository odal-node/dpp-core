//! [`RegistrySyncPort`] — the contract an EU Central Registry adapter implements.

use async_trait::async_trait;

use super::acknowledgement::SubmissionAcknowledgement;
use super::record::RegistryRecord;
use super::report::SubmissionReport;
use super::request::RegistrationRequest;
use crate::error::DppError;

/// Port trait for synchronising DPP records with the EU Central Registry.
///
/// No API specification is published. This trait is shaped by what IR (EU)
/// 2026/1778 and the registry's own guide establish about the flow, and by
/// nothing about the wire: that is the adapter's business.
///
/// # A registration happens in two steps, with two different handles
///
/// Submitting does not register anything. The registry accepts a submission for
/// validation and assigns it a **correlation identifier**; it rules on it
/// afterwards (Art. 8(7)), and only a successful ruling generates the passport's
/// **registration identifier** (Art. 8(8)). So:
///
/// - [`register`](Self::register) submits, and answers with the correlation
///   identifier;
/// - [`check_submission`](Self::check_submission) polls the submission by it,
///   and answers with the registration identifier once there is one;
/// - [`check_status`](Self::check_status) and
///   [`notify_transfer`](Self::notify_transfer) act on the registered record by
///   its registration identifier.
///
/// 🚨 **None of them takes this crate's `PassportId`.** The registry never
/// received one: a poll keyed on it asks the registry about a value it has no
/// record of, which is what this trait used to do.
///
/// # Ghost implementation
///
/// Until a deployment is enrolled, platform code can wire
/// [`GhostRegistrySync`](crate::ports::ghosts::GhostRegistrySync), which
/// accepts every submission under a placeholder handle and knows of no records.
#[async_trait]
pub trait RegistrySyncPort: Send + Sync {
    /// Submit one passport for registration.
    ///
    /// Called when a passport transitions from Draft to Published. The answer
    /// is an acknowledgement, not a registration: see
    /// [`SubmissionAcknowledgement`].
    async fn register(
        &self,
        request: RegistrationRequest,
    ) -> Result<SubmissionAcknowledgement, DppError>;

    /// What became of a submission, by the correlation identifier the registry
    /// assigned it.
    async fn check_submission(&self, correlation_id: &str) -> Result<SubmissionReport, DppError>;

    /// The current state of a registered passport, by its registration
    /// identifier (IR (EU) 2026/1778 Art. 8(8)).
    async fn check_status(&self, registration_id: &str) -> Result<RegistryRecord, DppError>;

    /// Update a registry record after a transfer of responsibility.
    ///
    /// Called when a product's responsible economic operator changes
    /// (e.g. remanufacturing, repurposing, import into a new market).
    ///
    /// `registration_id` is the registry's own identifier for this passport,
    /// from the ruling on its submission. Without it the registry has no way to
    /// know which record the handover refers to, so a caller that does not yet
    /// have one must wait rather than send an unattached notification.
    ///
    /// Takes the whole [`TransferRecord`](crate::transfer::TransferRecord)
    /// rather than just the incoming
    /// operator's identifier. A registry notification names **both** legal
    /// persons and carries the dual signatures that authorise the handover;
    /// passing only the new identifier left an adapter no way to express the
    /// outgoing operator or either signature, so it could only send empty
    /// strings for data the system had already collected.
    async fn notify_transfer(
        &self,
        record: &crate::transfer::TransferRecord,
        registration_id: &str,
    ) -> Result<RegistryRecord, DppError>;
}
