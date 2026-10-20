//! [`FacilitySnapshot`] — a by-value copy of the Annex III facility at passport creation.

use serde::{Deserialize, Serialize};

use crate::identifier::{ActorIdentifier, ActorIdentifierError, ActorPosition};

/// A self-contained snapshot of the ESPR Annex III facility a passport was
/// stamped with at creation.
///
/// Copied **by value** onto the passport so the published, signed record remains
/// complete for its full retention period even if the operator later retires the
/// source facility from their registry. Field shape matches
/// `dpp_registry::FacilityIdentifier`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FacilitySnapshot {
    /// Identifier scheme (e.g. `"gln"`, `"national"`).
    pub scheme: String,
    /// Identifier value (e.g. the 13-digit GLN) — the Annex III unique facility id.
    pub value: String,
    /// Human-readable facility name.
    pub name: String,
    /// ISO 3166-1 alpha-2 country code of the facility.
    pub country: String,
    /// Optional street address / location description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// The facility identifier typed by EN 18219 clause 6 scheme, beside the
    /// legacy [`scheme`](Self::scheme) and [`value`](Self::value) pair.
    ///
    /// `Option`, because this type is a persisted shape and the envelope rule is
    /// additive-only: a snapshot written before this field existed reads back as
    /// `None`, and is not written with the key at all, so its signed bytes do not
    /// move. `None` means not stated, never "has none". The legacy pair is kept
    /// for the same reason, since a reader that has not upgraded still reads it.
    ///
    /// Read an identifier with [`actor_identifier`](Self::actor_identifier), which
    /// uses this when it is present and reads the legacy pair otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<ActorIdentifier>,
}

impl FacilitySnapshot {
    /// This facility's identifier as an EN 18219 clause 6 identifier.
    ///
    /// The typed [`identifier`](Self::identifier) when the snapshot carries one.
    /// It is **not** checked against the legacy pair: when both are present the
    /// typed one is what the snapshot states. Otherwise the legacy pair is read
    /// through [`ActorIdentifier::from_legacy`], at a facility, so a `gln` is a
    /// physical location and a `national` code is
    /// [`Unmapped`](ActorIdentifierError::Unmapped).
    ///
    /// # Errors
    ///
    /// See [`ActorIdentifier::from_legacy`].
    pub fn actor_identifier(&self) -> Result<ActorIdentifier, ActorIdentifierError> {
        match &self.identifier {
            Some(identifier) => Ok(identifier.clone()),
            None => {
                ActorIdentifier::from_legacy(&self.scheme, &self.value, ActorPosition::Facility)
            }
        }
    }
}
