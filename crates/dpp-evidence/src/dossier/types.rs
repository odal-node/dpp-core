use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use dpp_domain::domain::transfer::TransferChain;
use serde::{Deserialize, Serialize};

use crate::audit::AuditEntry;

/// A JWS alongside the exact JSON payload it was signed over.
///
/// The engine-side signer applies transforms before signing (e.g. the
/// full-view payload forces `status` to `"active"`; the public-view payload
/// is a redacted projection). Rather than have this crate reimplement those
/// engine-internal transforms to reconstruct what *should* have been signed,
/// the dossier assembler — which already has that exact value in hand —
/// embeds it directly. Verification then only has to confirm the signature
/// covers *this* payload, not derive the payload itself.
///
/// `deny_unknown_fields`: an unrecognised member here fails deserialization
/// (exit 2 / malformed) rather than being silently dropped and still
/// verifying green — see [`crate::verify::verify_dossier_json`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SignedLayer {
    pub payload: serde_json::Value,
    pub jws: String,
}

/// Signed description of a dossier — the manifest JWS payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DossierManifest {
    /// Dossier wire format version, `"1"`.
    pub format_version: String,
    pub passport_id: String,
    /// The DID that signed the manifest, `full_view`, and `public_view` —
    /// the node operator's own identity. Transfer-chain signatures carry
    /// their own signer DIDs on each record instead.
    pub issuer_did: String,
    pub created_at: DateTime<Utc>,
    pub node_version: String,
    /// The `dpp-calc` ruleset version, when a determination ran. `None` for
    /// passthrough-only passports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruleset_version: Option<String>,
    /// member name -> hex SHA-256 over the JCS-canonicalised member content.
    /// Binds every dossier member into one atomic, tamper-evident unit — an
    /// attacker cannot swap in a genuinely-signed-but-stale member (e.g. an
    /// older audit trail that omits a later suspend event) without the
    /// manifest's own signature catching the mismatch.
    pub content_hashes: BTreeMap<String, String>,
}

/// A complete evidence dossier: everything needed to verify a passport's
/// full proof chain offline, with zero trust in the issuing node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DossierV1 {
    pub manifest: DossierManifest,
    pub manifest_jws: String,
    pub full_view: SignedLayer,
    pub public_view: SignedLayer,
    /// DID document snapshots, keyed by DID. Always contains at least
    /// `manifest.issuer_did`; may contain other operators' DIDs when a
    /// transfer chain is present.
    pub did_documents: BTreeMap<String, serde_json::Value>,
    /// Ordered ascending by timestamp (chain order).
    pub audit_entries: Vec<AuditEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfer_chain: Option<TransferChain>,
    /// Present iff the passport was deactivated (End-of-Life declared).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eol_event: Option<serde_json::Value>,
    /// Always `None` in v1 — the signed-checkpoint layer is not yet built.
    /// Present as a field (not omitted) so the format doesn't need a
    /// breaking version bump when checkpoints ship.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<serde_json::Value>,
    /// Always empty in v1 — `dpp-calc` invocation is not yet wired end to
    /// end (see the roadmap note on licensed factor data). Present as a
    /// field for the same forward-compatibility reason as `checkpoint`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calc_receipts: Vec<serde_json::Value>,
}
