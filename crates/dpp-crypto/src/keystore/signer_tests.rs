//! A key behind a [`KeySigner`] signs through every signer in this crate, and fails
//! through every one of them in its own words.

use serde_json::json;

use super::entry::{SigningMaterial, fingerprint_of};
use super::{KeyEntry, KeyRevoked, KeySigner, SignError};
use crate::cose::{CoseError, sign_sign1};
use crate::jws::algorithm::KeyAlgorithm;
use crate::jws::public_key::PublicKey;
use crate::jws::signer::sign_typed_with;
use crate::sd_jwt::SdJwtError;

const EVERY_ALGORITHM: [KeyAlgorithm; 4] = [
    KeyAlgorithm::Ed25519,
    KeyAlgorithm::EcdsaP256,
    KeyAlgorithm::EcdsaP384,
    KeyAlgorithm::EcdsaP521,
];

/// A signer that holds its key itself, as a device would: the entry never sees the secret.
struct Holding(SigningMaterial);

impl KeySigner for Holding {
    fn algorithm(&self) -> KeyAlgorithm {
        self.0.algorithm()
    }

    fn public_key(&self) -> PublicKey {
        self.0.public_key()
    }

    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, SignError> {
        Ok(self.0.sign(message))
    }
}

/// A signer whose device is gone.
struct Unreachable(PublicKey);

impl KeySigner for Unreachable {
    fn algorithm(&self) -> KeyAlgorithm {
        self.0.algorithm()
    }

    fn public_key(&self) -> PublicKey {
        self.0.clone()
    }

    fn sign(&self, _message: &[u8]) -> Result<Vec<u8>, SignError> {
        Err(SignError::Backend("the device is not connected".to_owned()))
    }
}

/// The same key, once in memory and once behind a signer.
fn local_and_behind_a_signer(algorithm: KeyAlgorithm) -> (KeyEntry, KeyEntry) {
    let material = SigningMaterial::generate(algorithm);
    let secret = material.secret();
    let public = material.public_key();
    let local = KeyEntry::new(material, fingerprint_of(&public), false);
    let again = SigningMaterial::from_secret(algorithm, &secret).expect("the same key");
    (
        local,
        KeyEntry::from_signer(Box::new(Holding(again)), false),
    )
}

fn unreachable_entry() -> KeyEntry {
    let public = SigningMaterial::generate(KeyAlgorithm::Ed25519).public_key();
    KeyEntry::from_signer(Box::new(Unreachable(public)), false)
}

/// What a caller reads off a key is the same whether the key is in memory or behind a
/// signer, and so is everything signed with it.
#[test]
fn a_key_behind_a_signer_is_the_same_key_to_every_signer() {
    for algorithm in EVERY_ALGORITHM {
        let (local, behind) = local_and_behind_a_signer(algorithm);
        let label = algorithm.jose_alg();

        assert_eq!(behind.algorithm(), local.algorithm(), "{label}");
        assert_eq!(behind.public_key(), local.public_key(), "{label}");
        assert_eq!(behind.fingerprint, local.fingerprint, "{label}");
        assert_eq!(
            behind.sign(b"message").expect("signs"),
            local.sign(b"message").expect("signs"),
            "{label}: the same signature"
        );

        let payload = json!({"product": "battery"});
        assert_eq!(
            sign_typed_with(&behind, &payload, "kid", None, None).expect("a JWS"),
            sign_typed_with(&local, &payload, "kid", None, None).expect("a JWS"),
            "{label}: the same JWS"
        );
        assert_eq!(
            sign_sign1(&behind, b"{}", b"kid", None, None).expect("a COSE_Sign1"),
            sign_sign1(&local, b"{}", b"kid", None, None).expect("a COSE_Sign1"),
            "{label}: the same COSE_Sign1"
        );
    }
}

/// A signer that cannot sign is refused in its own words, by the entry and by every
/// signer built on it, and is never taken for a revocation.
#[test]
fn a_signer_that_cannot_sign_is_refused_through_every_signer() {
    let key = unreachable_entry();
    let reason = "the device is not connected";

    assert_eq!(
        key.sign(b"message"),
        Err(SignError::Backend(reason.to_owned()))
    );

    let jws = sign_typed_with(&key, &json!({}), "kid", None, None).expect_err("a JWS");
    assert_eq!(
        jws.downcast_ref::<SignError>(),
        Some(&SignError::Backend(reason.to_owned()))
    );
    assert!(jws.downcast_ref::<KeyRevoked>().is_none(), "{jws}");

    assert_eq!(
        sign_sign1(&key, b"{}", b"kid", None, None),
        Err(CoseError::Signing(reason.to_owned()))
    );
    assert_eq!(
        SdJwtError::from(SignError::Backend(reason.to_owned())),
        SdJwtError::Signing(reason.to_owned())
    );
}

/// A revoked key is refused before its signer is asked, so a signer behind a device is
/// not woken for a signature nobody may use.
#[test]
fn a_revoked_key_is_refused_before_its_signer_is_asked() {
    struct MustNotSign(PublicKey);
    impl KeySigner for MustNotSign {
        fn algorithm(&self) -> KeyAlgorithm {
            self.0.algorithm()
        }
        fn public_key(&self) -> PublicKey {
            self.0.clone()
        }
        fn sign(&self, _message: &[u8]) -> Result<Vec<u8>, SignError> {
            panic!("a revoked key's signer was asked to sign");
        }
    }

    let public = SigningMaterial::generate(KeyAlgorithm::Ed25519).public_key();
    let fingerprint = fingerprint_of(&public);
    let key = KeyEntry::from_signer(Box::new(MustNotSign(public)), true);

    assert_eq!(
        key.sign(b"message"),
        Err(SignError::Revoked(KeyRevoked { fingerprint }))
    );
    assert!(matches!(
        sign_sign1(&key, b"{}", b"kid", None, None),
        Err(CoseError::KeyRevoked(_))
    ));
    assert!(matches!(
        SdJwtError::from(SignError::Revoked(KeyRevoked {
            fingerprint: String::new()
        })),
        SdJwtError::KeyRevoked(_)
    ));
}
