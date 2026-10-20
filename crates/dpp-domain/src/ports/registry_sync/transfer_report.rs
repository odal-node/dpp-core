//! [`TransferReport`] — what the registry answers a transfer notification with.

use serde::{Deserialize, Serialize};

/// What the registry said when a transfer of responsibility was notified to it.
///
/// A notification amends a record that already exists, so the answer to it is not
/// a record's state. [`RegistryRecord`](super::RegistryRecord) and
/// [`RegistryStatus`](super::RegistryStatus) say where a *registration* stands;
/// a refusal of a notification, or a reply that it is still being processed, is
/// neither. Answering with them forced an adapter to hide a refusal inside a
/// status the port had no name for, and left a caller unable to tell an
/// acceptance from a state nobody recognised.
///
/// One variant per thing a caller does next: close the notification, close it
/// because an earlier attempt already got through, send it again later, show the
/// refusal, or leave it open and say what the registry reported.
///
/// # Sending it again is safe only under a stable key
///
/// Three of these answers mean "ask again", and asking again re-sends the
/// notification. That is harmless exactly when the registry recognises the second
/// attempt as the first. The handover has a stable identity,
/// [`TransferRecord::transfer_id`](crate::transfer::TransferRecord::transfer_id),
/// so an adapter sends it as the registry's idempotency key. A replay is then
/// answered [`AlreadyHeld`](Self::AlreadyHeld), which is how a notification whose
/// first answer was lost, unreadable or unrecognised is settled without a second
/// one reaching the registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "outcome")]
#[non_exhaustive]
pub enum TransferReport {
    /// The registry took the notification and amended the record.
    Accepted,
    /// The registry already holds this notification.
    ///
    /// What a replayed idempotency key means: an earlier attempt reached the
    /// registry and its answer was lost, or was one the caller could not settle
    /// on. Sending it again would be wrong, and so would calling it a failure.
    AlreadyHeld,
    /// The registry has the notification and has not yet applied it.
    ///
    /// Not final. The caller sends it again later under the same key, and the
    /// replay settles it as [`AlreadyHeld`](Self::AlreadyHeld) or
    /// [`Accepted`](Self::Accepted) once the registry has finished.
    Processing,
    /// The registry refused the notification.
    ///
    /// Terminal: resending the same notification asks the same question. `reason`
    /// is what the registry said, or says that it said nothing.
    Refused {
        /// The registry's description of the refusal.
        reason: String,
    },
    /// The registry answered with a state this crate has no name for.
    ///
    /// Not a verdict. Reading it as `Accepted` would close a notification the
    /// registry may not have taken, which is a handover of responsibility the
    /// record never learns of; reading it as `Refused` would give up on one it may
    /// have. A caller keeps the notification open, sends it again under the same
    /// key, and shows the state as given. A state that never settles is for a
    /// person to look at.
    Unrecognised {
        /// The state exactly as the registry reported it.
        state: String,
    },
}

impl TransferReport {
    /// Whether the notification is settled, so it is not sent again.
    ///
    /// [`Processing`](Self::Processing) and [`Unrecognised`](Self::Unrecognised)
    /// are not: both end in a resend.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Accepted | Self::AlreadyHeld | Self::Refused { .. }
        )
    }
}
