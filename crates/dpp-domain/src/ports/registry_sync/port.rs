//! [`RegistrySyncPort`] — the contract an EU Central Registry adapter implements.

use async_trait::async_trait;

use super::acknowledgement::SubmissionAcknowledgement;
use super::lookup::SubmissionLookup;
use super::record::RegistryRecord;
use super::report::SubmissionReport;
use super::request::RegistrationRequest;
use super::transfer_report::TransferReport;
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
/// - [`find_submission`](Self::find_submission) recovers the correlation
///   identifier when a replayed key says the registry already holds the
///   submission and does not say under which handle;
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

    /// Find the submission a request made, when its correlation identifier is no
    /// longer held.
    ///
    /// For a caller that was told
    /// [`AlreadyHeld`](SubmissionAcknowledgement::AlreadyHeld): the registry has
    /// the submission and a replayed key does not say under which handle. An
    /// adapter that can read the registry's record of the operator's requests
    /// matches `request` to it by the unique product identifier the passport was
    /// submitted under, [`data_carrier_uri`](RegistrationRequest::data_carrier_uri),
    /// which is what the registry reports each passport against, and answers
    /// with the handle and what became of the submission.
    ///
    /// The registry's *unique product identifier* is that URL, not
    /// [`product_identifier`](RegistrationRequest::product_identifier), which is
    /// the EN 18219 clause 5 value the request also carries. The two are
    /// different values, so a match on `product_identifier` would never find the
    /// submission.
    ///
    /// 🚨 A match is by that identifier and nothing else, and an identifier is
    /// not unique to a passport: a successor can be submitted under its
    /// predecessor's. When more than one submission reports it the answer is
    /// [`Ambiguous`](SubmissionLookup::Ambiguous), never the nearest in time or
    /// position, which would give a passport another passport's registration.
    ///
    /// The default answers [`Unsupported`](SubmissionLookup::Unsupported), so an
    /// adapter written before this method existed still compiles and a caller
    /// treats it as it always did.
    async fn find_submission(
        &self,
        _request: &RegistrationRequest,
    ) -> Result<SubmissionLookup, DppError> {
        Ok(SubmissionLookup::Unsupported)
    }

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
    ///
    /// The answer is a [`TransferReport`], not a [`RegistryRecord`]: a
    /// notification amends a record, so what the registry says about it is
    /// whether it took the notification, not where a registration stands.
    ///
    /// 🚨 **An adapter sends [`record.transfer_id`](crate::transfer::TransferRecord::transfer_id)
    /// as the registry's idempotency key.** Three answers
    /// ([`Processing`](TransferReport::Processing),
    /// [`Unrecognised`](TransferReport::Unrecognised) and an error) end in a
    /// caller sending the notification again, and the key is what makes that the
    /// same notification and not a second one. A caller that retries over an
    /// adapter that does not send one is sending duplicates.
    async fn notify_transfer(
        &self,
        record: &crate::transfer::TransferRecord,
        registration_id: &str,
    ) -> Result<TransferReport, DppError>;
}
