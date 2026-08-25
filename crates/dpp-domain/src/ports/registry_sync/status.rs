//! [`RegistryStatus`] — where a record stands in the EU Central Registry.

use serde::{Deserialize, Serialize};

/// Status of a DPP record within the EU Central Registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum RegistryStatus {
    /// Registration submitted but not yet confirmed by the registry.
    Pending,
    /// Successfully registered and identifiers assigned.
    Registered,
    /// Registration rejected (e.g. missing fields, invalid operator).
    Rejected,
    // No `Transferred`: the registry has no such status. A transfer notification
    // amends an existing record, which stays `Registered` — whether the handover
    // was notified is the notification's own state, not the registration's, and
    // belongs on the transfer queue. The variant existed and was unreachable,
    // promising a status the registry never reports.
    /// Record suspended by a market surveillance authority.
    SuspendedByAuthority,
    /// Record withdrawn from service in the registry.
    ///
    /// Distinct from [`Self::Rejected`], which it used to be collapsed into.
    /// A rejection says our submission was defective and can be corrected; a
    /// deactivation says the record is no longer in service, which is not
    /// something resubmitting fixes. Reporting one as the other sends an
    /// operator after the wrong remedy.
    Deactivated,
}
