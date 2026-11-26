//! The VC-JWT envelope under every algorithm a key can be generated under: what each
//! one signs and authenticates as, and what a rotation across algorithms leaves
//! verifiable.

use base64::Engine as _;
use serde_json::Value;

use dpp_crypto::jws::KeyAlgorithm;

use crate::credential::{authenticate_access_credential, sign_access_credential};
use crate::did_builder::build_did_document;
use crate::test_support::{EVERY_ALGORITHM, temp_store, temp_store_with};

use super::super::jwt::envelope::{BASE_URL, ISSUER_DID, KEY_ID, credential_from};

/// The protected header of a compact JWS, read without checking anything.
fn header_of(jws: &str) -> Value {
    let part = jws.split('.').next().expect("a header");
    serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(part)
            .expect("base64url"),
    )
    .expect("JSON")
}

/// Each algorithm signs the credential under its own `alg`, and the DID document
/// publishes the key that authenticates it.
#[test]
fn every_algorithm_signs_a_credential_that_authenticates() {
    for algorithm in EVERY_ALGORITHM {
        let name = algorithm.jose_alg();
        let store = temp_store_with("jwt-alg", KEY_ID, algorithm);
        let doc = build_did_document(&store, BASE_URL, KEY_ID).expect("did document");
        let input = credential_from(ISSUER_DID);
        let jws = sign_access_credential(&input, &store, KEY_ID).expect("sign");

        assert_eq!(
            header_of(&jws)["alg"],
            name,
            "{name}: the header names the key's algorithm"
        );
        let credential = authenticate_access_credential(&jws, &doc)
            .unwrap_or_else(|e| panic!("{name}: must authenticate, got {e:?}"));
        assert_eq!(credential.issuer, ISSUER_DID, "{name}");
        assert_eq!(
            credential.credential_subject, input.credential_subject,
            "{name}: the subject id, role and product scope come back as signed"
        );
    }
}

/// Signing is stable under every algorithm: the same credential signs to the same
/// bytes. ECDSA nonces are deterministic (RFC 6979), and so is Ed25519.
#[test]
fn signing_is_stable_under_every_algorithm() {
    for algorithm in EVERY_ALGORITHM {
        let store = temp_store_with("jwt-stable", KEY_ID, algorithm);
        let credential = credential_from(ISSUER_DID);

        let first = sign_access_credential(&credential, &store, KEY_ID).expect("sign");
        let second = sign_access_credential(&credential, &store, KEY_ID).expect("sign again");

        assert_eq!(
            first,
            second,
            "{}: the same credential must sign to the same bytes",
            algorithm.jose_alg()
        );
    }
}

/// A credential signed before a rotation from Ed25519 to an EC key authenticates
/// against the document published after it, and so does one signed after.
#[test]
fn a_credential_signed_before_a_rotation_to_an_ec_key_authenticates_after_it() {
    for target in [
        KeyAlgorithm::EcdsaP256,
        KeyAlgorithm::EcdsaP384,
        KeyAlgorithm::EcdsaP521,
    ] {
        let name = target.jose_alg();
        let store = temp_store("jwt-rotate", KEY_ID);
        let before = sign_access_credential(&credential_from(ISSUER_DID), &store, KEY_ID)
            .expect("sign before the rotation");

        store
            .rotate_key_to(KEY_ID, target)
            .expect("rotate to an EC key");
        let after = sign_access_credential(&credential_from(ISSUER_DID), &store, KEY_ID)
            .expect("sign after the rotation");
        let doc = build_did_document(&store, BASE_URL, KEY_ID).expect("did document");

        for (when, jws) in [("before", &before), ("after", &after)] {
            authenticate_access_credential(jws, &doc).unwrap_or_else(|e| {
                panic!(
                    "{name}: a credential signed {when} the rotation must authenticate, got {e:?}"
                )
            });
        }
    }
}

/// A credential signed while the issuer's key rotates names the key that signed it,
/// and authenticates against the document published once the rotations stop.
///
/// `sign_access_credential` takes the `kid` and the signature from one load of the
/// key. With two reads, a rotation between them gave a credential the replaced key's
/// `kid` and the new key's signature, and the document refused it.
#[test]
fn credentials_signed_while_the_key_rotates_authenticate_against_the_document_published_after() {
    let store = temp_store("jwt-while-rotating", KEY_ID);
    let input = credential_from(ISSUER_DID);

    let credentials = crate::test_support::sign_while_rotating(&store, KEY_ID, || {
        sign_access_credential(&input, &store, KEY_ID).expect("sign while the key rotates")
    });
    let doc = build_did_document(&store, BASE_URL, KEY_ID).expect("did document");

    let kids: std::collections::BTreeSet<String> = credentials
        .iter()
        .map(|jws| {
            header_of(jws)["kid"]
                .as_str()
                .expect("the token names a kid")
                .to_owned()
        })
        .collect();
    assert!(
        kids.len() > 1,
        "the credentials were signed under more than one key: {}",
        kids.len()
    );
    for (i, jws) in credentials.iter().enumerate() {
        authenticate_access_credential(jws, &doc).unwrap_or_else(|e| {
            panic!(
                "credential {i} of {} names {} and does not authenticate: {e:?}",
                credentials.len(),
                header_of(jws)["kid"]
            )
        });
    }
}
