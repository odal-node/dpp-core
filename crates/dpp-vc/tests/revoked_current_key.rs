//! A key store whose current key is revoked.
//!
//! The key signs nothing, through any signer; the DID document and the SD-JWT VC
//! issuer metadata publish the same keys and neither publishes it; and
//! `KeyStore::revoke_and_rotate` gives the store a current key that signs again. The
//! store is the one way this state opens: a file written before key stores carried
//! an integrity tag, with the flag already set, sealed by `open_and_migrate`. The
//! same documents are handed to the W3C DID suite by `did_suite_input`.

use std::collections::BTreeSet;
use std::sync::Arc;

use serde_json::{Value, json};

use dpp_crypto::keystore::{KeyRevoked, KeyStore};
use dpp_domain::access::ProductGroupAccessPolicy;
use dpp_domain::ports::identity::IdentityPort;
use dpp_domain::{PassportId, ProductGroup};
use dpp_vc::credential::{authenticate_access_credential, sign_access_credential};
use dpp_vc::sd_jwt_vc::{Issuance, issue};
use dpp_vc::{
    CredentialBuilder, CredentialRole, DppCredentialSubject, LocalIdentityService,
    build_did_document, build_issuer_metadata, did_for, vct_for,
};

#[path = "key_store_support/legacy_store.rs"]
mod legacy_store;

use legacy_store::{KEY_ONE, KEY_TWO, PASSPHRASE, with_revoked_current};

const KEY_ID: &str = "issuer";
const BASE_URL: &str = "https://revoked-current.example";
const ISSUER: &str = "https://revoked-current.example";

fn open(path: &std::path::Path) -> Arc<KeyStore> {
    Arc::new(KeyStore::open_and_migrate(path, PASSPHRASE).expect("the store opens and is sealed"))
}

/// The thumbprint URIs the DID document lists as verification methods.
fn did_document_keys(doc: &Value) -> BTreeSet<String> {
    let prefix = format!("{}#", did_for(BASE_URL));
    doc.get("verificationMethod")
        .and_then(Value::as_array)
        .map(|methods| {
            methods
                .iter()
                .map(|m| {
                    let id = m["id"].as_str().expect("a method names its id");
                    id.strip_prefix(&prefix)
                        .expect("a method is the issuer's")
                        .to_owned()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The `kid`s the issuer metadata publishes, which are thumbprint URIs too.
fn metadata_keys(metadata: Option<&Value>) -> BTreeSet<String> {
    metadata
        .map(|m| {
            m["jwks"]["keys"]
                .as_array()
                .expect("a key set")
                .iter()
                .map(|k| k["kid"].as_str().expect("a key names its kid").to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// The thumbprint URI of the current key, as the store records it.
fn current_key(store: &KeyStore) -> String {
    store
        .public_key(KEY_ID)
        .expect("a current key")
        .thumbprint_uri()
        .expect("a valid public key")
}

fn access_credential() -> dpp_vc::DppAccessCredential {
    CredentialBuilder::new(
        did_for(BASE_URL),
        DppCredentialSubject {
            id: "did:web:repairer.example".into(),
            name: "Repairs GmbH".into(),
            role: CredentialRole::AuthorisedRepairer,
            country: "DE".into(),
            product_groups: vec!["battery".into()],
            product_categories: Vec::new(),
        },
    )
    .expires_in_days(30)
    .build()
}

fn sd_jwt_vc(store: &KeyStore) -> Result<dpp_crypto::sd_jwt::SdJwt, dpp_vc::SdJwtVcError> {
    let policy = ProductGroupAccessPolicy::for_passport("battery", "2.6.0")
        .expect("battery 2.6.0 schema is embedded");
    issue(
        store,
        KEY_ID,
        &json!({"productGroup": "battery", "schemaVersion": "2.6.0"}),
        &policy,
        &Issuance {
            issuer: ISSUER,
            vct: &vct_for(ProductGroup::Battery, "2.6.0"),
            issued_at: 1_789_000_000,
            holder_key: None,
        },
    )
}

/// Every signer of this workspace refuses the revoked current key, whichever way
/// it reaches the key.
async fn every_signer_refuses(store: &Arc<KeyStore>) {
    let key = store.load_key(KEY_ID).expect("a revoked key still loads");
    assert!(key.revoked, "the current key reads back revoked");
    let fingerprint = key.fingerprint.clone();

    let refused: KeyRevoked = key.sign(b"anything").expect_err("the key itself refuses");
    assert_eq!(refused.fingerprint, fingerprint);
    assert!(
        refused.to_string().contains("revoke_and_rotate"),
        "{refused}"
    );

    let jws = dpp_crypto::jws::sign(store, KEY_ID, &json!({}), "kid").expect_err("jws::sign");
    assert!(jws.downcast_ref::<KeyRevoked>().is_some(), "{jws}");
    let verify =
        dpp_crypto::jws::verify(store, KEY_ID, "e30.e30.AA").expect_err("jws::verify refuses");
    assert!(verify.downcast_ref::<KeyRevoked>().is_some(), "{verify}");
    assert!(matches!(
        dpp_crypto::cose::sign_sign1(&key, b"{}", b"kid", None, None),
        Err(dpp_crypto::cose::CoseError::KeyRevoked(_))
    ));

    let credential = sign_access_credential(&access_credential(), store, KEY_ID)
        .expect_err("sign_access_credential");
    assert!(
        credential.downcast_ref::<KeyRevoked>().is_some(),
        "{credential}"
    );

    let service = LocalIdentityService::new(Arc::clone(store), KEY_ID.into(), BASE_URL.into());
    let passport = service
        .sign_passport(PassportId::new(), &json!({"productGroup": "battery"}))
        .await
        .expect_err("sign_passport");
    assert!(passport.to_string().contains("revoked"), "{passport}");

    let sd_jwt = sd_jwt_vc(store).expect_err("sd_jwt_vc::issue");
    assert!(sd_jwt.to_string().contains("revoked"), "{sd_jwt}");

    let document = json!({
        "@context": ["https://www.w3.org/ns/credentials/v2"],
        "type": ["VerifiableCredential"],
        "issuer": ISSUER,
        "credentialSubject": {"id": "did:example:subject", "name": "Example"},
    });
    let kid = format!("{ISSUER}#key");
    for refused in [
        dpp_vc::jose::sign_credential(&document, &key, &kid, &[]).map(|_| ()),
        dpp_vc::jose::sign_credential_cose(&document, &key, &kid, &[]).map(|_| ()),
    ] {
        let refused = refused.expect_err("a jose signer refuses");
        assert!(refused.to_string().contains("revoked"), "{refused}");
    }
}

/// A revoked current key beside an archived key that is not revoked: the key signs
/// nothing, both documents publish the archived key and only it, the DID document
/// names no `authentication`, and `revoke_and_rotate` recovers.
#[tokio::test]
async fn a_revoked_current_key_signs_nothing_and_neither_document_publishes_it() {
    let path = with_revoked_current("revoked-current", KEY_ID, &KEY_ONE, Some(&KEY_TWO));
    let store = open(&path);
    let revoked = current_key(&store);

    every_signer_refuses(&store).await;

    let doc = build_did_document(&store, BASE_URL, KEY_ID).expect("the document builds");
    let metadata = build_issuer_metadata(&store, ISSUER, KEY_ID);
    let published = did_document_keys(&doc);
    assert_eq!(
        published,
        metadata_keys(metadata.as_ref()),
        "the two documents agree"
    );
    assert_eq!(published.len(), 1, "the archived key alone: {published:?}");
    assert!(
        !published.contains(&revoked),
        "the revoked key is published nowhere"
    );
    assert!(
        doc.get("authentication").is_none(),
        "with no current key, authentication is left out rather than empty"
    );
    assert_eq!(
        doc["assertionMethod"],
        json!(
            doc["verificationMethod"]
                .as_array()
                .expect("methods")
                .iter()
                .map(|m| m["id"].clone())
                .collect::<Vec<_>>()
        )
    );

    // A compromise rotation gives the store a current key that signs, and keeps the
    // revoked one out of both documents.
    store.revoke_and_rotate(KEY_ID).expect("revoke and rotate");
    let successor = current_key(&store);
    let token =
        sign_access_credential(&access_credential(), &store, KEY_ID).expect("the successor signs");
    let doc = build_did_document(&store, BASE_URL, KEY_ID).expect("the document builds");
    authenticate_access_credential(&token, &doc).expect("and its token authenticates");

    let published = did_document_keys(&doc);
    assert_eq!(
        published,
        metadata_keys(build_issuer_metadata(&store, ISSUER, KEY_ID).as_ref())
    );
    assert_eq!(
        published.len(),
        2,
        "the successor and the archived key: {published:?}"
    );
    assert!(published.contains(&successor) && !published.contains(&revoked));
    assert_eq!(
        doc["authentication"],
        json!([format!("{}#{successor}", did_for(BASE_URL))])
    );
    let _ = std::fs::remove_file(&path);
}

/// A store whose only key is revoked publishes no key at all: the DID document holds
/// its `@context` and `id` and nothing else, and there is no issuer metadata. A
/// hygiene rotation recovers too, and keeps the revoked key revoked.
#[tokio::test]
async fn a_store_whose_only_key_is_revoked_publishes_none() {
    let path = with_revoked_current("only-revoked", KEY_ID, &KEY_ONE, None);
    let store = open(&path);
    let revoked = current_key(&store);

    every_signer_refuses(&store).await;

    let doc = build_did_document(&store, BASE_URL, KEY_ID).expect("the document builds");
    let members: BTreeSet<&str> = doc
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(members, BTreeSet::from(["@context", "id"]), "{doc}");
    assert!(build_issuer_metadata(&store, ISSUER, KEY_ID).is_none());

    // `rotate_key` archives the revoked key as revoked: a rotation never makes a
    // revoked key valid again.
    store.rotate_key(KEY_ID).expect("rotate");
    let successor = current_key(&store);
    sign_access_credential(&access_credential(), &store, KEY_ID).expect("the successor signs");
    let doc = build_did_document(&store, BASE_URL, KEY_ID).expect("the document builds");
    let published = did_document_keys(&doc);
    assert_eq!(published, BTreeSet::from([successor]));
    assert_eq!(
        published,
        metadata_keys(build_issuer_metadata(&store, ISSUER, KEY_ID).as_ref())
    );
    assert!(
        store
            .archived_public_keys(KEY_ID)
            .iter()
            .all(|k| k.revoked && k.thumbprint_uri().expect("valid") == revoked),
        "the revoked key is archived revoked"
    );
    let _ = std::fs::remove_file(&path);
}
