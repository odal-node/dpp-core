//! eIDAS qualified electronic seal value objects — what a seal *is* once
//! produced, and what an adapter reports about its own capabilities.
//!
//! These are **persisted domain values**, not ports: [`SealedEnvelope`] is
//! serialised onto `Passport::seal` and travels on the wire. The trait that
//! produces one is the extension seam and lives in [`crate::ports::seal`],
//! which also carries the regulatory basis for what "qualified" requires.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─── Types ───────────────────────────────────────────────────────────────────

/// Which eIDAS sealing model the request should use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SealMode {
    /// Platform holds its own qualified seal; operators use delegated access.
    ///
    /// **The legal basis for this mode is not established by the registry
    /// rules.** Verified against the OJ text of IR 2026/1778: Art. 19(4) permits
    /// a verified economic operator to authorise a third party to perform
    /// *"registration actions in the registry"* on its behalf, provided that
    /// third party follows the verification process in accordance with Art. 5.
    /// That is delegated **registration**, and it says nothing about who may
    /// hold or use a qualified electronic seal.
    ///
    /// Art. 19(5) is likewise about data rather than seals: each verified
    /// economic operator *"shall be responsible for the data it submits to the
    /// Commission as manager of the registry and shall be considered as the
    /// controller of the data it submits"*.
    ///
    /// So delegation of registration is settled and delegation of sealing is
    /// not. Whether one party may hold a qualified seal covering content another
    /// party authored is a question under eIDAS and the applicable delegated
    /// act, not one these articles answer — and the mechanics moving would not
    /// move the responsibility either way.
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

impl SealCapabilities {
    /// Whether these capabilities cover what `req` asks for.
    ///
    /// Defined once, here, so every adapter answers the question the same way.
    /// An adapter that rolled its own check would be free to disagree with the
    /// capabilities it advertises, which is the disagreement the check exists to
    /// make impossible.
    ///
    /// Both axes must match. A provider that produces the right envelope format
    /// under the wrong certificate holder has not produced what was asked for:
    /// the mode decides *whose* attestation the seal is, which is a statement
    /// about responsibility rather than a serialisation detail.
    pub fn can_produce(&self, req: &SealRequest) -> bool {
        self.supported_formats.contains(&req.sig_format) && self.supported_modes.contains(&req.mode)
    }
}

/// Result of verifying a `SealedEnvelope`.
#[derive(Debug, Clone)]
pub struct SealVerification {
    /// Whether the seal cryptographically verifies.
    pub valid: bool,
    /// True if this was a ghost/placeholder seal (always unverified in production).
    pub placeholder: bool,
}

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
