//! Wire types for a signed, versioned ruleset bundle.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Signed description of a ruleset bundle — the JWS payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RulesetManifest {
    /// Channel bundle version, e.g. `"2026-Q3.1"`.
    pub bundle_version: String,
    /// When this bundle's rules take effect.
    pub effective_date: DateTime<Utc>,
    /// EU-act citations this bundle encodes (audit trail for the change).
    #[serde(default)]
    pub act_citations: Vec<String>,
    /// Sector → schema version this bundle references (never forks schemas).
    #[serde(default)]
    pub schema_versions: BTreeMap<String, String>,
    /// Hex SHA-256 over the JCS-canonicalised `content`.
    pub content_sha256: String,
}

/// A signed bundle on the wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedBundle {
    /// Compact EdDSA JWS over the manifest, signed by the publisher key.
    pub manifest_jws: String,
    /// The ruleset payload the manifest commits to.
    pub content: serde_json::Value,
}

/// A bundle that passed both signature and hash checks. Only constructible via
/// [`crate::bundle::verify_bundle`], so holding one is proof it verified.
#[derive(Debug, Clone)]
pub struct VerifiedRuleset {
    /// The verified manifest.
    pub manifest: RulesetManifest,
    /// The verified content.
    pub content: serde_json::Value,
}

impl VerifiedRuleset {
    /// The active bundle version (surfaced on `/health`, stamped into provenance).
    #[must_use]
    pub fn version(&self) -> &str {
        &self.manifest.bundle_version
    }
}

/// Why a bundle was refused. Verification is fail-closed — any of these keeps
/// the caller on its current ruleset.
#[derive(Debug, thiserror::Error)]
pub enum RulesetError {
    /// The manifest JWS did not verify under the pinned publisher key.
    #[error("bundle signature invalid or not signed by the pinned publisher key")]
    BadSignature,
    /// `content` does not hash to the value in the signed manifest.
    #[error("bundle content hash mismatch — content does not match the signed manifest")]
    ContentHashMismatch,
    /// The bundle was structurally malformed.
    #[error("malformed bundle: {0}")]
    Malformed(String),
}
