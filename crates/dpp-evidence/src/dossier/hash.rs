use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use super::types::DossierV1;

/// Canonical SHA-256 (hex) of a JSON value (RFC 8785 / JCS bytes).
///
/// Exposed so the assembler builds the exact same hash a verifier will
/// later recompute and check against — one hash function, two call sites,
/// mirroring `dpp-rules::bundle::verify::content_hash`.
#[must_use]
pub fn content_hash(value: &serde_json::Value) -> String {
    let bytes = serde_jcs::to_vec(value).expect("JCS canonicalisation is infallible");
    hex::encode(Sha256::digest(&bytes))
}

/// Compute the `content_hashes` map for a dossier's members, in the shape
/// [`super::DossierManifest::content_hashes`] expects. Both the assembler (to
/// build the manifest before signing) and the verifier (to recompute and
/// compare) call this on the same dossier shape, so the two can never drift.
#[must_use]
pub fn compute_content_hashes(dossier: &DossierV1) -> BTreeMap<String, String> {
    let mut hashes = BTreeMap::new();
    hashes.insert(
        "fullView".to_string(),
        content_hash(&dossier.full_view.payload),
    );
    hashes.insert(
        "publicView".to_string(),
        content_hash(&dossier.public_view.payload),
    );
    hashes.insert(
        "auditEntries".to_string(),
        content_hash(
            &serde_json::to_value(&dossier.audit_entries).expect("audit entries serialise"),
        ),
    );
    if let Some(chain) = &dossier.transfer_chain {
        hashes.insert(
            "transferChain".to_string(),
            content_hash(&serde_json::to_value(chain).expect("TransferChain serialises")),
        );
    }
    if let Some(eol) = &dossier.eol_event {
        hashes.insert("eolEvent".to_string(), content_hash(eol));
    }
    hashes
}
