//! [`SealRequest`] — what a caller asks a seal adapter to produce.

use serde::{Deserialize, Serialize};

use super::conformance_level::SealConformanceLevel;
use super::credential_ref::SealCredentialRef;
use super::envelope::SealEnvelope;
use super::format::SealFormat;
use super::mode::SealMode;

/// Input to a seal operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealRequest {
    /// SHA-256 hex digest of the payload to seal.
    pub payload_hash: String,
    /// Which eIDAS sealing model to use.
    pub mode: SealMode,
    /// Reference to the QTSP-held credential (not key material).
    pub key_ref: SealCredentialRef,
    /// Desired AdES envelope format. JAdES is recommended.
    pub sig_format: SealFormat,
    /// How much validation material the seal must carry.
    ///
    /// Defaults to [`SealConformanceLevel::BaselineLt`] on the wire — the
    /// weakest level that survives certificate expiry. A default that did not
    /// would quietly hand a retention-locked passport a seal with a shelf life.
    #[serde(default = "default_conformance_level")]
    pub conformance_level: SealConformanceLevel,
    /// Where the seal sits relative to the bytes it covers.
    ///
    /// Defaults to [`SealEnvelope::Detached`], which is what a request built
    /// from a payload hash means: the caller already holds the bytes and wants
    /// an attestation over them.
    #[serde(default = "default_envelope")]
    pub envelope: SealEnvelope,
}

fn default_conformance_level() -> SealConformanceLevel {
    SealConformanceLevel::BaselineLt
}

fn default_envelope() -> SealEnvelope {
    SealEnvelope::Detached
}
