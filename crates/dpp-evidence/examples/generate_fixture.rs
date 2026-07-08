//! Generates a small, fully valid, deterministically-signed dossier with a
//! transfer chain and EOL event — used as a black-box fixture for external
//! verifiers (e.g. `dpp-engine`'s `odal verify` CLI tests). Run with:
//!
//!     cargo run -p dpp-evidence --example generate_fixture > valid-dossier.json

use std::collections::BTreeMap;

use base64::Engine;
use chrono::{TimeZone, Utc};
use dpp_domain::domain::passport::PassportId;
use dpp_domain::domain::transfer::{
    OperatorRole, ResponsibleOperator, TransferChain, TransferReason, TransferRecord,
};
use dpp_evidence::audit::AuditEntry;
use dpp_evidence::dossier::{DossierManifest, DossierV1, SignedLayer, compute_content_hashes};
use ed25519_dalek::{Signer, SigningKey};
use uuid::Uuid;

fn sign(signing_key: &SigningKey, payload: &serde_json::Value) -> String {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header = b64.encode(serde_json::to_vec(&serde_json::json!({"alg": "EdDSA"})).unwrap());
    let body = b64.encode(serde_jcs::to_vec(payload).unwrap());
    let signing_input = format!("{header}.{body}");
    let sig = signing_key.sign(signing_input.as_bytes());
    format!("{signing_input}.{}", b64.encode(sig.to_bytes()))
}

fn did_doc_for(signing_key: &SigningKey, did: &str) -> serde_json::Value {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let x = b64.encode(signing_key.verifying_key().to_bytes());
    serde_json::json!({
        "id": did,
        "verificationMethod": [{
            "id": format!("{did}#root"),
            "type": "JsonWebKey2020",
            "publicKeyJwk": { "kty": "OKP", "crv": "Ed25519", "x": x },
        }],
        "assertionMethod": [format!("{did}#root")],
    })
}

fn operator(did: &str, name: &str) -> ResponsibleOperator {
    ResponsibleOperator {
        did: did.to_owned(),
        name: name.to_owned(),
        role: OperatorRole::Manufacturer,
        eu_operator_id: None,
        country: "DE".into(),
    }
}

fn chain_entries(mut entries: Vec<AuditEntry>) -> Vec<AuditEntry> {
    let mut prev = String::new();
    for e in &mut entries {
        let h = e.chain_hash(&prev);
        e.prev_hash = Some(prev.clone());
        e.entry_hash = Some(h.clone());
        prev = h;
    }
    entries
}

fn entry(passport_id: &str, action: &str, ts: chrono::DateTime<Utc>) -> AuditEntry {
    AuditEntry {
        id: Uuid::now_v7(),
        passport_id: passport_id.to_owned(),
        actor: "demo-user".into(),
        action: action.into(),
        previous_status: None,
        new_status: Some("active".into()),
        metadata: None,
        timestamp: ts,
        prev_hash: None,
        entry_hash: None,
    }
}

fn main() {
    let issuer_key = SigningKey::from_bytes(&[42u8; 32]);
    let issuer_did = "did:web:demo.odal-node.io".to_string();

    let passport_id = "01HXYZDEMO0000000000000001".to_string();
    let full_payload = serde_json::json!({
        "id": passport_id,
        "productName": "Demo Battery Pack",
        "status": "active",
        "sector": "battery",
    });
    let public_payload = serde_json::json!({
        "id": passport_id,
        "productName": "Demo Battery Pack",
        "sector": "battery",
    });

    let now = Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap();
    let audit_entries = chain_entries(vec![
        entry(&passport_id, "created", now),
        entry(
            &passport_id,
            "published",
            now + chrono::Duration::minutes(5),
        ),
    ]);

    let from_key = SigningKey::from_bytes(&[7u8; 32]);
    let to_key = SigningKey::from_bytes(&[8u8; 32]);
    let from_did = "did:web:from.example".to_string();
    let to_did = "did:web:to.example".to_string();

    let mut record = TransferRecord {
        transfer_id: Uuid::now_v7(),
        passport_id: PassportId::new(),
        from_operator: operator(&from_did, "From Operator GmbH"),
        to_operator: operator(&to_did, "To Operator SARL"),
        reason: TransferReason::Sale,
        from_signature: None,
        to_signature: None,
        initiated_at: now + chrono::Duration::hours(1),
        completed_at: Some(now + chrono::Duration::hours(2)),
        rejected_at: None,
        cancelled_at: None,
        notes: Some("demo fixture transfer".into()),
    };
    let transfer_payload = record.signing_payload();
    record.from_signature = Some(sign(&from_key, &transfer_payload));
    record.to_signature = Some(sign(&to_key, &transfer_payload));

    let transfer_chain = TransferChain {
        passport_id: record.passport_id,
        original_operator: operator(&from_did, "From Operator GmbH"),
        transfers: vec![record],
    };

    let mut did_documents = BTreeMap::new();
    did_documents.insert(issuer_did.clone(), did_doc_for(&issuer_key, &issuer_did));
    did_documents.insert(from_did.clone(), did_doc_for(&from_key, &from_did));
    did_documents.insert(to_did.clone(), did_doc_for(&to_key, &to_did));

    let mut dossier = DossierV1 {
        manifest: DossierManifest {
            format_version: "1".into(),
            passport_id: passport_id.clone(),
            issuer_did: issuer_did.clone(),
            created_at: now + chrono::Duration::hours(3),
            node_version: "0.7.0-demo".into(),
            ruleset_version: None,
            content_hashes: BTreeMap::new(),
        },
        manifest_jws: String::new(),
        full_view: SignedLayer {
            payload: full_payload.clone(),
            jws: sign(&issuer_key, &full_payload),
        },
        public_view: SignedLayer {
            payload: public_payload.clone(),
            jws: sign(&issuer_key, &public_payload),
        },
        did_documents,
        audit_entries,
        transfer_chain: Some(transfer_chain),
        eol_event: None,
        checkpoint: None,
        calc_receipts: Vec::new(),
    };

    dossier.manifest.content_hashes = compute_content_hashes(&dossier);
    let manifest_value = serde_json::to_value(&dossier.manifest).unwrap();
    dossier.manifest_jws = sign(&issuer_key, &manifest_value);

    println!("{}", serde_json::to_string_pretty(&dossier).unwrap());
}
