//! Golden cross-tests between `dpp-crypto`'s JWS signer/verifier and
//! `dpp-evidence`'s vendored copy.
//!
//! `dpp-evidence` cannot depend on `dpp-crypto` directly (it breaks the
//! wasm32-unknown-unknown build — see `dpp-evidence/src/jws/mod.rs`'s doc
//! comment), so its verify path is a maintained duplicate. These tests are
//! the drift guard: sign with the real signer, verify with both
//! implementations, and assert they agree on every tamper class.

use dpp_crypto::identity::did_builder::build_did_document;
use dpp_crypto::jws::{self as core_jws};
use dpp_crypto::keystore::KeyStore;

fn open_store() -> KeyStore {
    let path = std::env::temp_dir().join(format!("jws-cross-test-{}.json", uuid::Uuid::now_v7()));
    KeyStore::open(&path, "test-pass").expect("open keystore")
}

// ── 1. Round-trip: sign with dpp-crypto, verify with dpp-evidence ─────────

#[test]
fn round_trip_signs_with_core_verifies_with_evidence() {
    let store = open_store();
    store.generate_key("root").expect("generate key");
    let payload = serde_json::json!({"passportId": "p1", "status": "active"});

    let jws = core_jws::sign(&store, "root", &payload).expect("sign");
    let did_doc = build_did_document(&store, "cross-test.example.com", "root").expect("did doc");
    let key_b64 = dpp_evidence::jws::extract_primary_public_key(&did_doc)
        .expect("dpp-evidence must find the primary key in a dpp-crypto-built DID document");

    assert!(dpp_evidence::jws::verify_jws(&jws, &key_b64).unwrap());
    assert!(dpp_evidence::jws::verify_jws_content(&jws, &key_b64, &payload).unwrap());

    // And the reverse direction, for good measure: dpp-crypto's own verifier
    // over the same JWS/key.
    assert!(core_jws::verify_jws(&jws, &key_b64).unwrap());
}

// ── 2. Tamper matrix — both implementations must reject every class ───────

#[test]
fn tamper_matrix_both_implementations_reject_every_class() {
    let store = open_store();
    store.generate_key("root").expect("generate key");
    let payload = serde_json::json!({"a": 1, "b": "two"});
    let jws = core_jws::sign(&store, "root", &payload).expect("sign");
    let did_doc = build_did_document(&store, "cross-test.example.com", "root").expect("did doc");
    let key_b64 = dpp_evidence::jws::extract_primary_public_key(&did_doc).unwrap();

    let parts: Vec<&str> = jws.splitn(3, '.').collect();
    assert_eq!(parts.len(), 3);

    // Flip the first character of a base64url segment to another valid
    // base64url character — same length, so it stays decodable (unlike
    // appending a character, which can break the base64 padding/length
    // invariant and turn "tampered" into "malformed input" instead).
    fn flip_first_char(segment: &str) -> String {
        let replacement = if segment.starts_with('A') { 'B' } else { 'A' };
        format!("{replacement}{}", &segment[1..])
    }

    // Flipped payload byte.
    let tampered_payload = format!("{}.{}.{}", parts[0], flip_first_char(parts[1]), parts[2]);
    assert!(!core_jws::verify_jws(&tampered_payload, &key_b64).unwrap());
    assert!(!dpp_evidence::jws::verify_jws(&tampered_payload, &key_b64).unwrap());

    // Flipped signature byte.
    let tampered_sig = format!("{}.{}.{}", parts[0], parts[1], flip_first_char(parts[2]));
    assert!(!core_jws::verify_jws(&tampered_sig, &key_b64).unwrap());
    assert!(!dpp_evidence::jws::verify_jws(&tampered_sig, &key_b64).unwrap());

    // `alg` swapped to `none`.
    let b64 = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        serde_json::to_vec(&serde_json::json!({"alg": "none"})).unwrap(),
    );
    let alg_none = format!("{b64}.{}.{}", parts[1], parts[2]);
    assert!(!core_jws::verify_jws(&alg_none, &key_b64).unwrap());
    assert!(!dpp_evidence::jws::verify_jws(&alg_none, &key_b64).unwrap());

    // `alg` swapped to an unrelated algorithm name.
    let b64 = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        serde_json::to_vec(&serde_json::json!({"alg": "HS256"})).unwrap(),
    );
    let alg_other = format!("{b64}.{}.{}", parts[1], parts[2]);
    assert!(!core_jws::verify_jws(&alg_other, &key_b64).unwrap());
    assert!(!dpp_evidence::jws::verify_jws(&alg_other, &key_b64).unwrap());

    // Sanity: the untampered original still verifies under both.
    assert!(core_jws::verify_jws(&jws, &key_b64).unwrap());
    assert!(dpp_evidence::jws::verify_jws(&jws, &key_b64).unwrap());
}

// ── 3. Key-resolution parity across rotation ───────────────────────────────

#[test]
fn dpp_evidence_resolves_a_rotation_archived_key_the_same_way_as_dpp_crypto() {
    let store = open_store();
    store.generate_key("root").expect("generate key");
    let payload = serde_json::json!({"before": "rotation"});
    let old_jws = core_jws::sign(&store, "root", &payload).expect("sign with pre-rotation key");

    // Hygiene rotation: the old key is archived but stays valid (not
    // revoked) — `jws::verify(store, key_id, ..)` is not rotation-aware (it
    // just loads whatever is *currently* stored at `key_id`), so the
    // rotation-aware path both `LocalIdentityService::verify_signature` and
    // the resolver/CLI actually use is: build the DID document (which lists
    // archived, non-revoked keys too), extract the JWS's `kid`, and find the
    // matching key by fingerprint. Reproduced inline here with dpp-crypto's
    // own public functions so this is a true apples-to-apples comparison.
    store.rotate_key("root").expect("rotate");
    let did_doc_after = build_did_document(&store, "cross-test.example.com", "root")
        .expect("did doc after rotation");

    let kid = core_jws::extract_kid_from_jws(&old_jws).expect("jws carries a kid");
    let core_key_b64 = core_jws::extract_key_by_fingerprint(&did_doc_after, &kid)
        .or_else(|| core_jws::extract_primary_public_key(&did_doc_after))
        .expect("dpp-crypto must find the archived key by fingerprint");
    assert!(core_jws::verify_jws(&old_jws, &core_key_b64).expect("core verify"));

    // dpp-evidence must resolve the same archived key the same way, using
    // only the published DID document — no store access.
    let evidence_kid =
        dpp_evidence::jws::extract_kid_from_jws(&old_jws).expect("jws carries a kid");
    assert_eq!(
        evidence_kid, kid,
        "both implementations must extract the same kid"
    );
    let evidence_key_b64 =
        dpp_evidence::jws::extract_key_by_fingerprint(&did_doc_after, &evidence_kid)
            .expect("dpp-evidence must find the archived key by fingerprint");
    assert_eq!(
        evidence_key_b64, core_key_b64,
        "both implementations must resolve the same key material"
    );
    assert!(dpp_evidence::jws::verify_jws_content(&old_jws, &evidence_key_b64, &payload).unwrap());
}

// ── 4. Same signature verifies under both implementations ─────────────────
//
// `KeyStore::generate_key` always uses `OsRng` (no seeded-key constructor is
// exposed publicly), so a literal committed golden JWS string isn't
// reachable through the public API. This instead pins the property that
// actually matters: for the *same* signature, the two independently
// maintained verifiers must agree — if either implementation's algorithm
// silently drifts (canonicalisation, key encoding, signing-input assembly),
// this test is what catches it.

#[test]
fn same_signature_verifies_identically_under_both_implementations() {
    let store = open_store();
    store.generate_key("root").expect("generate key");
    let payload = serde_json::json!({
        "id": "01HXYZGOLDEN0000000000001",
        "productName": "Golden Vector Widget",
        "status": "active",
        "materials": ["lithium", "aluminium"],
    });
    let jws = core_jws::sign(&store, "root", &payload).expect("sign");
    let did_doc = build_did_document(&store, "cross-test.example.com", "root").expect("did doc");
    let key_b64 = dpp_evidence::jws::extract_primary_public_key(&did_doc).unwrap();

    let core_result = core_jws::verify_jws(&jws, &key_b64).expect("core verify");
    let evidence_result = dpp_evidence::jws::verify_jws(&jws, &key_b64).expect("evidence verify");
    assert!(
        core_result && evidence_result,
        "both implementations must agree the golden signature verifies"
    );

    let evidence_content_result = dpp_evidence::jws::verify_jws_content(&jws, &key_b64, &payload)
        .expect("evidence content verify");
    assert!(evidence_content_result, "content-binding must also hold");
}
