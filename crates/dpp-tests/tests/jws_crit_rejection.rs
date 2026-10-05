//! Every signature check built on `dpp-crypto` refuses a JWS whose protected
//! header carries `crit` (RFC 7515 clause 4.1.11).
//!
//! `dpp-crypto` has two verifiers, and its own tests cover both of them
//! directly. This file covers what is built on top of them, because a check
//! added to a verifier protects a caller only while that caller still reaches
//! it: the snapshot bound, the access credential, the local identity service,
//! an SD-JWT VC, and a ruleset bundle verified through an adapter over the
//! crate's verifier.
//!
//! Each path gets a control and a refusal. Both tokens are built by hand with a
//! genuine signature, and they differ only in `crit`, so a refusal can only be
//! the `crit`. A control that failed would make every refusal here meaningless.

use std::collections::BTreeMap;
use std::sync::Arc;

use base64::Engine;
use chrono::{TimeZone, Utc};
use dpp_crypto::jws::{canonicalize, verifier::verify_jws};
use dpp_crypto::keystore::KeyStore;
use dpp_crypto::sd_jwt::SdJwt;
use dpp_domain::ProductGroup;
use dpp_domain::access::ProductGroupAccessPolicy;
use dpp_domain::ports::identity::IdentityPort;
use dpp_rules::bundle::{
    AcceptancePolicy, JwsVerify, RulesetError, RulesetManifest, SignedBundle, content_hash,
    verify_bundle,
};
use dpp_tests::fixtures::make_subject;
use dpp_vc::credential::authenticate_access_credential;
use dpp_vc::sd_jwt_vc::{issue, verify};
use dpp_vc::{
    CredentialBuilder, CredentialRole, LocalIdentityService, SD_JWT_VC_TYP, SdJwtVcError,
    SnapshotBound, VerificationResult, build_did_document, vct_for, verify_snapshot_bound,
};
use ed25519_dalek::Signer;
use serde_json::{Value, json};

const KEY_ID: &str = "crit-key";
const BASE_URL: &str = "https://authority.example";
/// What `build_did_document` derives from [`BASE_URL`].
const ISSUER_DID: &str = "did:web:authority.example";

/// The protected header with no `crit`: what every producer in this workspace
/// writes.
fn plain() -> Value {
    json!({"alg": "EdDSA"})
}

/// The same header plus a critical extension this workspace does not implement.
fn crit() -> Value {
    json!({"alg": "EdDSA", "crit": ["b64"], "b64": false})
}

fn temp_store(label: &str) -> KeyStore {
    let path = std::env::temp_dir().join(format!("test-{label}-{}.json", uuid::Uuid::now_v7()));
    let store = KeyStore::open(path, "test-pass").expect("open store");
    store.generate_key(KEY_ID).expect("generate key");
    store
}

fn public_key_b64(store: &KeyStore) -> String {
    let key = store.load_key(KEY_ID).expect("load key");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key.verifying_key.to_bytes())
}

/// A compact JWS carrying a genuine signature over `header` and `payload`, with
/// the `kid` this workspace issues added so a verifier selects the key the way
/// it does for its own tokens: the DID URL of the key's verification method.
fn forge(store: &KeyStore, header: &Value, payload: &[u8]) -> String {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let key = store.load_key(KEY_ID).expect("load key");
    let thumbprint = store
        .public_key(KEY_ID)
        .expect("public key")
        .thumbprint_uri()
        .expect("thumbprint");
    let mut header = header.clone();
    header["kid"] = json!(format!("{ISSUER_DID}#{thumbprint}"));
    let signing_input = format!(
        "{}.{}",
        b64.encode(serde_json::to_vec(&header).expect("header serialises")),
        b64.encode(payload),
    );
    let signature = key.signing_key.sign(signing_input.as_bytes());
    format!("{signing_input}.{}", b64.encode(signature.to_bytes()))
}

// ── Snapshot bound ───────────────────────────────────────────────────────────

fn snapshot(store: &KeyStore, header: &Value) -> Value {
    let mut document = json!({
        "id": "0192f3c0-0000-7000-8000-000000000000",
        "status": "active",
        "asOf": "2026-10-01T00:00:00Z",
        "validUntil": "2026-12-01T00:00:00Z",
    });
    let proof = forge(
        store,
        header,
        &canonicalize(&document).expect("document canonicalises"),
    );
    document["snapshotJwsSignature"] = json!(proof);
    document
}

#[test]
fn a_snapshot_proof_with_crit_proves_nothing() {
    let store = temp_store("crit-snapshot");
    let key = public_key_b64(&store);
    let now = Utc.with_ymd_and_hms(2026, 10, 5, 0, 0, 0).unwrap();

    assert!(
        matches!(
            verify_snapshot_bound(&snapshot(&store, &plain()), &key, now),
            SnapshotBound::Current { .. }
        ),
        "control: the same snapshot without crit must verify"
    );
    assert!(matches!(
        verify_snapshot_bound(&snapshot(&store, &crit()), &key, now),
        SnapshotBound::Unproven(_)
    ));
}

// ── Access credential ────────────────────────────────────────────────────────

fn access_credential(store: &KeyStore, header: &Value) -> String {
    let credential = CredentialBuilder::new(
        ISSUER_DID.into(),
        make_subject(
            "did:web:repairer.example",
            "Repairs GmbH",
            CredentialRole::AuthorisedRepairer,
            vec!["battery".into()],
        ),
    )
    .expires_in_days(30)
    .build();
    forge(
        store,
        header,
        &serde_json::to_vec(&credential).expect("credential serialises"),
    )
}

#[test]
fn an_access_credential_with_crit_does_not_authenticate() {
    let store = temp_store("crit-credential");
    let document = build_did_document(&store, BASE_URL, KEY_ID).expect("did document");

    assert!(
        authenticate_access_credential(&access_credential(&store, &plain()), &document).is_ok(),
        "control: the same credential without crit must authenticate"
    );
    assert!(matches!(
        authenticate_access_credential(&access_credential(&store, &crit()), &document),
        Err(VerificationResult::InvalidSignature(_))
    ));
}

// ── Local identity service ───────────────────────────────────────────────────

#[tokio::test]
async fn the_local_identity_service_refuses_a_signature_with_crit() {
    let store = Arc::new(temp_store("crit-service"));
    let service = LocalIdentityService::new(store.clone(), KEY_ID.into(), BASE_URL.into());
    let payload = json!({"id": "0192f3c0-0000-7000-8000-000000000000", "status": "active"});
    let canonical = canonicalize(&payload).expect("payload canonicalises");

    let control = forge(&store, &plain(), &canonical);
    assert!(
        service
            .verify_signature(&control, &payload)
            .await
            .expect("verifies"),
        "control: the same signature without crit must verify"
    );
    let refused = forge(&store, &crit(), &canonical);
    assert!(
        !service
            .verify_signature(&refused, &payload)
            .await
            .expect("verifies"),
    );
}

// ── SD-JWT VC ────────────────────────────────────────────────────────────────

/// A credential issued the normal way, whose issuer-signed JWT is then re-signed
/// under `header` with its payload and disclosures untouched.
fn sd_jwt_vc_under(store: &KeyStore, vct: &str, header: &Value) -> String {
    let payload = json!({
        "id": "018f3a4c-0000-7000-8000-000000000001",
        "productGroup": "battery",
        "schemaVersion": "2.6.0",
        "batchId": "BATCH-7781",
    });
    let policy = ProductGroupAccessPolicy::for_passport("battery", "2.6.0")
        .expect("battery 2.6.0 schema is embedded");
    let issued = issue(
        store,
        KEY_ID,
        &payload,
        &policy,
        "https://passports.operator.example",
        vct,
        1_789_000_000,
    )
    .expect("issue");

    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let jwt_payload = b64
        .decode(issued.jwt().split('.').nth(1).expect("payload segment"))
        .expect("payload decodes");
    let mut header = header.clone();
    header["typ"] = json!(SD_JWT_VC_TYP);
    let jwt = forge(store, &header, &jwt_payload);
    SdJwt::new(jwt, issued.disclosures().to_vec()).serialise()
}

#[test]
fn an_sd_jwt_vc_with_crit_is_refused() {
    let store = temp_store("crit-sd-jwt-vc");
    let key = public_key_b64(&store);
    let vct = vct_for(ProductGroup::Battery, "2.6.0");
    let now = Utc.timestamp_opt(1_789_000_060, 0).unwrap();

    assert!(
        verify(&sd_jwt_vc_under(&store, &vct, &plain()), &key, &vct, now).is_ok(),
        "control: the same credential without crit must verify"
    );
    assert!(matches!(
        verify(&sd_jwt_vc_under(&store, &vct, &crit()), &key, &vct, now),
        Err(SdJwtVcError::BadSignature)
    ));
}

// ── Ruleset bundle ───────────────────────────────────────────────────────────

/// The adapter a consumer writes: `verify_bundle` delegates the signature check
/// to the caller, and the natural implementation is this crate's verifier.
struct OverDppCrypto;

impl JwsVerify for OverDppCrypto {
    fn verify_eddsa(&self, jws: &str, public_key_b64: &str) -> Result<bool, RulesetError> {
        verify_jws(jws, public_key_b64).map_err(|e| RulesetError::Malformed(e.to_string()))
    }
}

fn bundle(store: &KeyStore, header: &Value) -> SignedBundle {
    let content = json!({"rules": []});
    let manifest = RulesetManifest {
        bundle_version: "2026-Q3.1".to_owned(),
        effective_date: Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap(),
        act_citations: Vec::new(),
        schema_versions: BTreeMap::new(),
        content_sha256: content_hash(&content).expect("content hashes"),
    };
    SignedBundle {
        manifest_jws: forge(
            store,
            header,
            &serde_json::to_vec(&manifest).expect("manifest serialises"),
        ),
        content,
    }
}

#[test]
fn a_ruleset_bundle_whose_manifest_has_crit_is_refused() {
    let store = temp_store("crit-bundle");
    let key = public_key_b64(&store);
    let policy = AcceptancePolicy {
        now: Utc.with_ymd_and_hms(2026, 10, 5, 0, 0, 0).unwrap(),
        in_force: None,
    };

    assert!(
        verify_bundle(&bundle(&store, &plain()), &key, &OverDppCrypto, &policy).is_ok(),
        "control: the same bundle without crit must verify"
    );
    assert!(matches!(
        verify_bundle(&bundle(&store, &crit()), &key, &OverDppCrypto, &policy),
        Err(RulesetError::BadSignature)
    ));
}
