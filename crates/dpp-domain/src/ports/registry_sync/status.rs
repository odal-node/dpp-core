//! [`RegistryStatus`] — where a registered passport stands in the EU Central Registry.

use serde::{Deserialize, Serialize};

/// Status of a registered passport within the EU Central Registry.
///
/// A status belongs to a **record**, which exists only once a submission has
/// succeeded. Whether a submission is still being checked, or was refused, is
/// the submission's state and is reported by
/// [`SubmissionReport`](super::SubmissionReport): `Pending` and `Rejected` used
/// to sit here, describing records that a refused submission never creates.
///
/// No list of record states is published, and IR (EU) 2026/1778 implies more
/// than these: **Art. 4(4)** has a passport's status *"updated accordingly"*
/// when its operator's verification lapses, and **Art. 10(1)** reflects every
/// modification and deletion in the status of the registration. A state this
/// type does not name arrives as [`Unrecognised`](Self::Unrecognised), with
/// the registry's own words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
#[non_exhaustive]
pub enum RegistryStatus {
    /// Registered and in service.
    Registered,
    // No `Transferred`: the registry has no such status. A transfer notification
    // amends an existing record, which stays `Registered` — whether the handover
    // was notified is the notification's own state, not the registration's, and
    // belongs on the transfer queue. The variant existed and was unreachable,
    // promising a status the registry never reports.
    /// Record suspended by a market surveillance authority.
    SuspendedByAuthority,
    /// Record withdrawn from service in the registry.
    ///
    /// Not a refusal. A refusal says a submission was defective and can be
    /// corrected; a deactivation says the record is no longer in service, which
    /// is not something resubmitting fixes. Reporting one as the other sends an
    /// operator after the wrong remedy.
    Deactivated,
    /// A state outside the ones above, exactly as reported.
    ///
    /// A caller must not map it onto one of the others: read as `Registered` it
    /// reports a record that may be suspended, and read as `Deactivated` it
    /// tells an operator a product is withdrawn when it may not be.
    Unrecognised(String),
}

impl RegistryStatus {
    /// The serialised value, spelled out so a renamed variant cannot change it.
    #[must_use]
    pub fn wire_str(&self) -> &str {
        match self {
            Self::Registered => "REGISTERED",
            Self::SuspendedByAuthority => "SUSPENDED_BY_AUTHORITY",
            Self::Deactivated => "DEACTIVATED",
            Self::Unrecognised(value) => value,
        }
    }
}

impl From<String> for RegistryStatus {
    fn from(value: String) -> Self {
        match value.as_str() {
            "REGISTERED" => Self::Registered,
            "SUSPENDED_BY_AUTHORITY" => Self::SuspendedByAuthority,
            "DEACTIVATED" => Self::Deactivated,
            _ => Self::Unrecognised(value),
        }
    }
}

impl From<RegistryStatus> for String {
    fn from(status: RegistryStatus) -> Self {
        match status {
            RegistryStatus::Unrecognised(value) => value,
            known => known.wire_str().to_owned(),
        }
    }
}
