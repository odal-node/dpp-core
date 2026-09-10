//! [`SealedEnvelope`] — the produced seal and the provenance it carries.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::conformance_level::SealConformanceLevel;
use super::format::SealFormat;

/// A completed qualified seal envelope returned by the QTSP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealedEnvelope {
    /// AdES format of this seal value.
    pub format: SealFormat,
    /// Base64-encoded seal value as returned by the QTSP.
    pub seal_value: String,
    /// Optional reference to the signing certificate chain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signing_cert_ref: Option<String>,
    /// The baseline level this seal was **requested** at.
    ///
    /// **A record of what was asked for, not proof of what arrived** — the same
    /// standing as `signingCertRef`, which names the certificate the seal claims
    /// rather than one anybody verified. What the bytes actually carry is read
    /// out of the envelope by a validator, and the two agreeing is the
    /// cross-check.
    ///
    /// # Why it is stored at all
    ///
    /// [`SealConformanceLevel`] documents why the level is on the request: a
    /// `BaselineB` seal stops verifying when its signing certificate expires,
    /// the seal is bought once, and the document it covers is retention-locked,
    /// so the choice cannot be corrected afterwards. All of that argues the
    /// level is the most consequential property of a seal — and it was the one
    /// property the envelope did not record.
    ///
    /// Without it, [`SealConformanceLevel::survives_certificate_expiry`] cannot
    /// be answered for any individual passport, and a node whose configuration
    /// moved over time (a development `B`, then a hosted `T`, then `LT`) leaves
    /// behind seals that are indistinguishable in storage. Recovering the answer
    /// then means parsing every `.p7s` by hand.
    ///
    /// `None` for an envelope written before this field existed, or for a seal
    /// restored from a backup or produced elsewhere. `None` means **not
    /// recorded** — never that the level was low.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conformance_level: Option<SealConformanceLevel>,
    /// Timestamp when the seal was created.
    ///
    /// **This node's clock at the moment the backend answered — not a trusted
    /// timestamp**, and not necessarily the instant the signature was formed. A
    /// seal only carries an independently established signing time from
    /// [`SealConformanceLevel::BaselineT`] upwards, where a timestamp authority
    /// attests it; at `BaselineB` there is no such token anywhere in the
    /// envelope, so this field is an unattested claim by the party that bought
    /// the seal.
    ///
    /// Named `sealed_at` rather than `signed_at` for that reason. Anything
    /// resting on *when* the seal was made must read the timestamp token out of
    /// the seal, not this field.
    pub sealed_at: DateTime<Utc>,
    /// True when this envelope was produced by `GhostSeal` and has no legal validity.
    pub placeholder: bool,
}
