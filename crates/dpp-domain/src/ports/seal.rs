//! Port trait for eIDAS qualified electronic sealing.
//!
//! ESPR Article 13 establishes the EU Central Registry, which became
//! operational on **20 July 2026**; its operating rules are Commission
//! Implementing Regulation (EU) 2026/1778.
//!
//! ⚠️ COMPLIANCE-PIN PENDING (watchlist 🟠): IR 2026/1778 has not been read
//! against the OJ text here. Treat its article-level detail as unconfirmed —
//! including whether a qualified seal is the required registration credential,
//! and whether a service provider may act for an operator. Do not state either
//! as enacted law.
//!
//! ## What "qualified" actually requires
//!
//! Verified against the OJ text of Regulation (EU) No 910/2014. A **qualified
//! electronic seal** is a three-part conjunction (**Art. 3(27)**):
//!
//! 1. an *advanced* electronic seal (**Art. 36**), **and**
//! 2. created by a *qualified electronic seal creation device* (**Art. 3(32)**),
//!    **and**
//! 3. based on a *qualified certificate* issued by a QTSP (**Art. 3(30)**,
//!    **Annex III**).
//!
//! An adapter that produces an advanced seal over a qualified certificate does
//! **not** satisfy this: the creation-device leg is separate and independently
//! required. **Annex III(j)** makes it machine-detectable — the certificate must
//! indicate, in a form suitable for automated processing, that the creation data
//! resides in such a device. The payoff is **Art. 35(2)**: a qualified seal
//! carries a presumption of integrity of the data and of correctness of its
//! origin.
//!
//! Two operating models exist:
//! - **Provider seal (delegated):** the platform holds its own qualified seal;
//!   operators register via delegated access without their own eIDAS credentials.
//! - **Operator seal:** the operator obtains and manages their own qualified seal.
//!
//! The real adapter calls a QTSP over the CSC API (Cloud Signature Consortium)
//! and lives in `dpp-engine`. Until a QTSP integration is configured,
//! `GhostSeal` returns clearly-synthetic envelopes so registration code can be
//! written and tested against this contract today.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::error::DppError;

// ─── Types ───────────────────────────────────────────────────────────────────

/// Which eIDAS sealing model the request should use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SealMode {
    /// Platform holds its own qualified seal; operators use delegated access.
    ///
    /// 🟠 Whether the registry admits a registration performed by a service
    /// provider acting for an economic operator is **unconfirmed** — see the
    /// module docs. This variant models the possibility; it does not assert the
    /// mode is available.
    ProviderSeal,
    /// Operator holds and manages their own qualified seal.
    OperatorSeal,
}

/// AdES signature/seal format family.
///
/// JAdES is the primary format: JSON-native, built on JWS (RFC 7515), and
/// the natural fit for DPP payloads. The others are modelled for completeness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum SealFormat {
    /// JSON Advanced Electronic Signatures (ETSI TS 119 182-1) — primary path.
    Jades,
    /// PDF Advanced Electronic Signatures.
    Pades,
    /// CMS Advanced Electronic Signatures (binary/CMS).
    Cades,
    /// XML Advanced Electronic Signatures.
    Xades,
}

/// A CSC-style reference to a QTSP-held credential. Never contains key material.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealCredentialRef {
    /// Identifier of the Qualified Trust Service Provider.
    pub qtsp_id: String,
    /// Credential identifier within the QTSP (CSC `credentialID`).
    pub credential_id: String,
}

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
}

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
    /// Timestamp when the seal was created.
    pub sealed_at: DateTime<Utc>,
    /// True when this envelope was produced by `GhostSeal` and has no legal validity.
    pub placeholder: bool,
}

/// Which seal formats and modes an adapter supports.
#[derive(Debug, Clone)]
pub struct SealCapabilities {
    pub supported_formats: Vec<SealFormat>,
    pub supported_modes: Vec<SealMode>,
}

/// Result of verifying a `SealedEnvelope`.
#[derive(Debug, Clone)]
pub struct SealVerification {
    /// Whether the seal cryptographically verifies.
    pub valid: bool,
    /// True if this was a ghost/placeholder seal (always unverified in production).
    pub placeholder: bool,
}

// ─── Port Trait ──────────────────────────────────────────────────────────────

/// Port trait for applying and verifying eIDAS qualified electronic seals.
///
/// Implementations live in `dpp-engine` and call a QTSP over the CSC API.
/// Until a QTSP is configured, wire `GhostSeal` so registration code compiles
/// and runs against a stable contract.
#[async_trait]
pub trait SealPort: Send + Sync {
    /// Apply a qualified seal to the given payload hash.
    async fn seal(&self, req: SealRequest) -> Result<SealedEnvelope, DppError>;

    /// Verify a previously produced seal envelope.
    async fn verify(&self, env: &SealedEnvelope) -> Result<SealVerification, DppError>;

    /// Report which formats and modes this adapter supports.
    fn capabilities(&self) -> SealCapabilities;
}

// ─── Ghost implementation (development / pre-QTSP) ───────────────────────────

/// No-op implementation for use before a QTSP integration is configured.
///
/// Returns synthetic envelopes marked `placeholder: true`. All operations
/// succeed but perform no network I/O and carry no legal validity.
pub use crate::ports::ghosts::GhostSeal;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_format_serde_round_trips() {
        for fmt in [
            SealFormat::Jades,
            SealFormat::Pades,
            SealFormat::Cades,
            SealFormat::Xades,
        ] {
            let json = serde_json::to_string(&fmt).unwrap();
            let back: SealFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(fmt, back);
        }
    }

    #[test]
    fn seal_mode_serde_round_trips() {
        for mode in [SealMode::ProviderSeal, SealMode::OperatorSeal] {
            let json = serde_json::to_string(&mode).unwrap();
            let back: SealMode = serde_json::from_str(&json).unwrap();
            assert_eq!(mode, back);
        }
    }
}
