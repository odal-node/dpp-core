//! The `did:web` document: its shape, its identifiers, and what rotation and
//! revocation do to it.

use base64::Engine;

use crate::did_builder::build_did_document;
use crate::test_support::temp_store;

#[test]
fn did_document_has_required_fields() {
    let store = temp_store("did-fields", "acme");
    let doc = build_did_document(&store, "https://example.com", "acme").expect("build");

    assert!(doc["@context"].is_array(), "@context must be array");
    assert!(doc["id"].is_string(), "id must be string");
    assert!(
        doc["verificationMethod"].is_array(),
        "verificationMethod must be array"
    );
    assert!(
        doc["assertionMethod"].is_array(),
        "assertionMethod must be array"
    );
    assert!(
        doc["authentication"].is_array(),
        "authentication must be array"
    );
}

#[test]
fn did_id_is_pathless_did_web() {
    let store = temp_store("did-id", "widgets-inc");
    let doc =
        build_did_document(&store, "https://identity.odal-node.io", "widgets-inc").expect("build");
    let id = doc["id"].as_str().expect("id is string");
    assert_eq!(id, "did:web:identity.odal-node.io");
}

#[test]
fn the_current_key_is_listed_first() {
    let store = temp_store("did-primary", "t-primary");
    let doc = build_did_document(&store, "https://id.example.com", "t-primary").expect("build");
    let current = store
        .public_key("t-primary")
        .expect("key")
        .thumbprint_uri()
        .expect("thumbprint");

    assert_eq!(
        doc["verificationMethod"][0]["id"],
        format!("did:web:id.example.com#{current}"),
        "the first verification method is the current key"
    );
}

#[test]
fn archived_keys_appear_as_secondary_methods_after_rotation() {
    let store = temp_store("did-rotate", "t-rotate");
    store.archive_key("t-rotate").expect("archive 1");
    store.generate_key("t-rotate").expect("new key 1");
    store.archive_key("t-rotate").expect("archive 2");
    store.generate_key("t-rotate").expect("new key 2");

    let doc = build_did_document(&store, "https://id.example.com", "t-rotate").expect("build");
    let vms = doc["verificationMethod"].as_array().expect("array");
    assert_eq!(
        vms.len(),
        3,
        "should have 3 verification methods after 2 rotations"
    );

    // Each is named by its own key, so the three identifiers are distinct and
    // none of them is a position.
    let ids: std::collections::BTreeSet<&str> = vms
        .iter()
        .map(|vm| {
            let id = vm["id"].as_str().expect("id");
            let kid = vm["publicKeyJwk"]["kid"].as_str().expect("kid");
            assert_eq!(id, format!("did:web:id.example.com#{kid}"));
            id
        })
        .collect();
    assert_eq!(ids.len(), 3, "every key has its own identifier");
}

/// A revoked key must not appear in the DID document at all.
#[test]
fn revoked_key_is_excluded_from_did_document() {
    let store = temp_store("did-revoke", "t-rev");

    // Hygiene rotation: old key archived but still valid → 2 methods.
    store.rotate_key("t-rev").expect("rotate");
    let doc = build_did_document(&store, "https://id.example.com", "t-rev").expect("build");
    assert_eq!(
        doc["verificationMethod"].as_array().unwrap().len(),
        2,
        "hygiene-archived key remains published"
    );

    // Compromise rotation: previous current key archived + revoked.
    store.revoke_and_rotate("t-rev").expect("revoke+rotate");
    let doc = build_did_document(&store, "https://id.example.com", "t-rev").expect("build");
    let vms = doc["verificationMethod"].as_array().unwrap();
    let am = doc["assertionMethod"].as_array().unwrap();
    assert_eq!(vms.len(), 2, "revoked key must be excluded, got {vms:?}");
    assert_eq!(am.len(), 2, "revoked key must not be an assertionMethod");
}

/// C-1 regression: VM type and @context must agree.
/// `JsonWebKey` (publicKeyJwk) is defined by the Controlled Identifiers context.
/// The older `JsonWebKey2020` belongs to the jws-2020 context, and
/// `Ed25519VerificationKey2020` (publicKeyMultibase) to ed25519-2020; neither
/// belongs here.
#[test]
fn did_document_context_and_vm_type_are_consistent() {
    let store = temp_store("did-ctx", "t-ctx");
    let doc = build_did_document(&store, "https://id.example.com", "t-ctx").expect("build");

    let contexts: Vec<&str> = doc["@context"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(
        contexts,
        [
            "https://www.w3.org/ns/did/v1",
            "https://www.w3.org/ns/cid/v1"
        ],
        "JsonWebKey VMs need the Controlled Identifiers context"
    );

    let vms = doc["verificationMethod"].as_array().unwrap();
    for vm in vms {
        assert_eq!(
            vm["type"].as_str().unwrap(),
            "JsonWebKey",
            "all VMs must be JsonWebKey to match the declared context"
        );
        assert!(
            vm.get("publicKeyJwk").is_some(),
            "JsonWebKey VM must carry publicKeyJwk"
        );
        assert!(
            vm.get("publicKeyMultibase").is_none(),
            "publicKeyMultibase belongs to Multikey, not JsonWebKey"
        );
    }
}

/// A verification method is identified by its own key's thumbprint, and its JWK
/// carries that same value as `kid`, so a reader never has to count.
#[test]
fn a_verification_method_is_identified_by_its_thumbprint() {
    let store = temp_store("did-id", "t-id");
    let doc = build_did_document(&store, "https://id.example.com", "t-id").expect("build");

    let vm = &doc["verificationMethod"][0];
    let thumbprint = store
        .public_key("t-id")
        .expect("key")
        .thumbprint_uri()
        .expect("thumbprint");

    assert_eq!(
        vm["id"],
        format!("did:web:id.example.com#{thumbprint}"),
        "the identifier is the DID, then the key's thumbprint URI"
    );
    assert_eq!(vm["publicKeyJwk"]["kid"], thumbprint);
    assert_eq!(vm["publicKeyJwk"]["alg"], "EdDSA");
    assert_eq!(doc["authentication"][0], vm["id"]);
    assert_eq!(doc["assertionMethod"][0], vm["id"]);
}

/// G-6: DID document golden vector — key material correctly encoded as JWK.
///
/// Pins that `verificationMethod[0].publicKeyJwk` has the correct JWK shape and
/// that `x` is exactly the base64url-encoded raw 32-byte verifying key from the
/// keystore. This is the cross-check that the DID builder does not corrupt or
/// mis-encode key material.
#[test]
fn did_document_jwk_x_matches_keystore_public_key() {
    let store = temp_store("did-g6", "g6-key");
    let loaded = store.load_key("g6-key").expect("load key");
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;

    let doc = build_did_document(&store, "https://id.example.com", "g6-key").expect("build");
    let vms = doc["verificationMethod"].as_array().unwrap();
    let primary_jwk = &vms[0]["publicKeyJwk"];

    assert_eq!(
        primary_jwk["kty"].as_str().unwrap(),
        "OKP",
        "kty must be OKP (RFC 8037)"
    );
    assert_eq!(
        primary_jwk["crv"].as_str().unwrap(),
        "Ed25519",
        "crv must be Ed25519"
    );

    // Pin: x must be base64url(verifying_key_bytes), not re-encoded or truncated.
    let expected_x = b64.encode(loaded.verifying_key.as_bytes());
    assert_eq!(
        primary_jwk["x"].as_str().unwrap(),
        expected_x,
        "publicKeyJwk.x must equal base64url(verifying_key.as_bytes()) from keystore"
    );

    // No private key material in JWK.
    assert!(
        primary_jwk.get("d").is_none(),
        "private key parameter 'd' must never appear in a published JWK"
    );
}

/// G-6: full DID Core structural validity on a realistic keystore (primary +
/// one hygiene-archived key + one revoked-and-rotated key), the exact
/// scenario the release review asked for. Goes beyond the narrower
/// `did_document_jwk_x_matches_keystore_public_key` (key-material shape) and
/// `did_document_context_and_vm_type_are_consistent` (C-1) checks above by
/// validating the document as a whole, including **referential integrity**
/// between `authentication`/`assertionMethod` and `verificationMethod` —
/// a real DID Core requirement that nothing else here checks: an
/// `authentication`/`assertionMethod` entry that doesn't resolve to an
/// `id` actually present in `verificationMethod` is a malformed DID
/// document, full stop, independent of any JSON-LD context resolution.
#[test]
fn did_document_is_structurally_valid_did_core_after_rotation_and_revocation() {
    let store = temp_store("did-structural", "g6-rotate");

    // Hygiene rotation: old key archived but still valid.
    store.rotate_key("g6-rotate").expect("hygiene rotate");
    // Compromise rotation: now-previous key archived + revoked.
    store.revoke_and_rotate("g6-rotate").expect("revoke+rotate");

    let doc = build_did_document(&store, "https://id.example.com", "g6-rotate").expect("build");

    // `id` is a well-formed pathless did:web identifier.
    let id = doc["id"].as_str().expect("id must be a string");
    assert!(
        id.starts_with("did:web:") && !id.contains('/'),
        "id must be a pathless did:web DID, got {id}"
    );

    // `@context` is a non-empty array of absolute https URIs.
    let contexts = doc["@context"].as_array().expect("@context must be array");
    assert!(!contexts.is_empty(), "@context must not be empty");
    for c in contexts {
        let s = c.as_str().expect("@context entries must be strings");
        assert!(
            s.starts_with("https://"),
            "@context entry must be an absolute https URI, got {s}"
        );
    }

    // `verificationMethod` entries are individually well-formed and scoped
    // to this DID (revoked key already excluded — covered elsewhere, but
    // re-asserted here as part of "the whole document is valid").
    let vms = doc["verificationMethod"]
        .as_array()
        .expect("verificationMethod must be array");
    assert_eq!(vms.len(), 2, "revoked key excluded, hygiene-archived kept");
    let mut seen_ids = std::collections::HashSet::new();
    for vm in vms {
        let vm_id = vm["id"].as_str().expect("VM id must be a string");
        assert!(
            vm_id.starts_with(&format!("{id}#")),
            "VM id must be a fragment of the document id, got {vm_id}"
        );
        assert!(seen_ids.insert(vm_id), "duplicate VM id: {vm_id}");
        assert_eq!(
            vm["controller"].as_str().unwrap(),
            id,
            "controller must equal the document id"
        );
        assert!(vm["type"].is_string(), "VM must declare a type");
        assert!(
            vm["publicKeyJwk"].is_object(),
            "VM must carry publicKeyJwk key material"
        );
    }

    // Referential integrity: every authentication/assertionMethod entry
    // must resolve to a verificationMethod actually present in the document.
    for rel in ["authentication", "assertionMethod"] {
        let refs = doc[rel]
            .as_array()
            .unwrap_or_else(|| panic!("{rel} must be an array"));
        assert!(!refs.is_empty(), "{rel} must not be empty");
        for r in refs {
            let r_id = r
                .as_str()
                .unwrap_or_else(|| panic!("{rel} entries must be strings"));
            assert!(
                seen_ids.contains(r_id),
                "{rel} entry '{r_id}' does not resolve to any verificationMethod id \
                 (dangling reference — a malformed DID document)"
            );
        }
    }
}
