use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// The `prev_hash` of the first (genesis) entry in a passport's chain.
pub const GENESIS_PREV_HASH: &str = "";

/// A single immutable audit record for a passport state change.
///
/// Entries are append-only at the storage layer (the engine's Postgres
/// schema raises on any `UPDATE`/`DELETE`), making the trail tamper-evident
/// independent of this crate. `#[serde(deny_unknown_fields)]` — an unknown
/// field here must fail loudly, not silently vanish from an otherwise-valid
/// content hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuditEntry {
    /// Unique identifier for this audit record.
    pub id: Uuid,
    /// The passport this entry is for (stringified UUID for forward-compat).
    pub passport_id: String,
    /// Who triggered this change. The engine stamps this from `AuthContext`;
    /// this crate only needs the resulting string.
    pub actor: String,
    /// Machine-readable action code, e.g. `"create"`, `"publish"`, `"archive"`.
    pub action: String,
    /// Passport status before the transition, if applicable.
    pub previous_status: Option<String>,
    /// Passport status after the transition, if applicable.
    pub new_status: Option<String>,
    /// Optional structured metadata (e.g. field diffs, a stamped EOL event,
    /// or — for a `"published"` entry — the exact payloads that were signed;
    /// see `dpp-vault`'s `publish.rs`/`evidence.rs`).
    pub metadata: Option<serde_json::Value>,
    /// Wall-clock timestamp of the operation (UUIDv7 source; sub-millisecond ordered).
    pub timestamp: DateTime<Utc>,
    /// Hash-chain link to the previous entry in this passport's chain.
    /// `""`/`None` for the genesis entry. Set by the storage layer on append.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev_hash: Option<String>,
    /// SHA-256 (hex) over the JCS-canonicalised content of this entry folded
    /// with `prev_hash` — the chain link the next entry points back to. Set
    /// by the storage layer on append; `None` on an in-memory entry not yet
    /// persisted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_hash: Option<String>,
}

impl AuditEntry {
    /// Construct an audit entry from an action and its actor.
    ///
    /// `id` is a new UUIDv7 so entries are time-ordered within a passport.
    /// Takes a plain actor string — the engine's `AuthContext`-aware call
    /// sites pass `&auth.user_id` (single-tenant: no operator scope to
    /// include, DECISION-0002).
    pub fn new(
        passport_id: &str,
        action: &str,
        actor: impl Into<String>,
        previous_status: Option<&str>,
        new_status: Option<&str>,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            passport_id: passport_id.to_owned(),
            actor: actor.into(),
            action: action.to_owned(),
            previous_status: previous_status.map(|s| s.to_owned()),
            new_status: new_status.map(|s| s.to_owned()),
            metadata: None,
            timestamp: Utc::now(),
            prev_hash: None,
            entry_hash: None,
        }
    }

    /// Attach structured metadata to this entry (builder-style).
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// The chain hash for this entry given its predecessor's hash: SHA-256 (hex)
    /// over the JCS-canonicalised content **and** `prev_hash`. Excludes the
    /// `prev_hash`/`entry_hash` columns themselves (prev is folded in as
    /// `prevHash`). Deterministic — the same content + prev always hashes equal.
    #[must_use]
    pub fn chain_hash(&self, prev_hash: &str) -> String {
        let canonical = serde_json::json!({
            "id": self.id,
            "passportId": self.passport_id,
            "actor": self.actor,
            "action": self.action,
            "previousStatus": self.previous_status,
            "newStatus": self.new_status,
            "metadata": self.metadata,
            "timestamp": self.timestamp,
            "prevHash": prev_hash,
        });
        let bytes = serde_jcs::to_vec(&canonical)
            .expect("JCS canonicalisation of audit content is infallible");
        hex::encode(Sha256::digest(&bytes))
    }
}
