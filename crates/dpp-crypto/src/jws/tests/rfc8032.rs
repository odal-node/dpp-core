//! RFC 8032 clause 7.1's Ed25519 test vectors, checked against `ed25519-dalek`,
//! the implementation every signature here is made and verified with.
//!
//! Nothing in this crate implements the curve: signing and strict verification
//! are delegated. The other tests here sign with that library and verify with
//! it, so a library that derived the wrong public key or produced a signature
//! other implementations reject would pass them all. These compare against the
//! keys and signatures the RFC publishes, which the library did not produce.
//!
//! What they establish, for the version locked in `Cargo.lock`: the public key
//! derives from the secret, signing is deterministic and yields the RFC's
//! signature, and `verify_strict` accepts it. `verify_strict` is the check
//! [`super::verifier::verify_jws`] makes.
//!
//! The vectors are the five of clause 7.1, including the 1023-byte message.

use ed25519_dalek::{Signature, Signer, SigningKey};

use super::common::vectors;

fn bytes<const N: usize>(hex_text: &str) -> [u8; N] {
    hex::decode(hex_text)
        .expect("hex")
        .try_into()
        .expect("the length the RFC gives")
}

#[test]
fn every_rfc_vector_derives_signs_and_verifies() {
    for v in vectors() {
        let message = hex::decode(&v.message).expect("hex message");
        let key = SigningKey::from_bytes(&bytes::<32>(v.secret));

        assert_eq!(
            hex::encode(key.verifying_key().to_bytes()),
            v.public,
            "{}: public key",
            v.name
        );

        let signature = key.sign(&message);
        assert_eq!(
            hex::encode(signature.to_bytes()),
            v.signature,
            "{}: signature",
            v.name
        );

        let published = Signature::from_bytes(&bytes::<64>(v.signature));
        assert!(
            key.verifying_key()
                .verify_strict(&message, &published)
                .is_ok(),
            "{}: the RFC's own signature must verify strictly",
            v.name
        );
    }
}

/// A signature is for one message under one key. The RFC's signature over one
/// vector's message must not verify another message, or another vector's key.
#[test]
fn an_rfc_signature_verifies_nothing_else() {
    let all = vectors();
    let (first, second) = (&all[1], &all[2]);
    let signature = Signature::from_bytes(&bytes::<64>(first.signature));
    let first_key = SigningKey::from_bytes(&bytes::<32>(first.secret)).verifying_key();
    let second_key = SigningKey::from_bytes(&bytes::<32>(second.secret)).verifying_key();

    let other_message = hex::decode(&second.message).expect("hex message");
    assert!(first_key.verify_strict(&other_message, &signature).is_err());

    let message = hex::decode(&first.message).expect("hex message");
    assert!(second_key.verify_strict(&message, &signature).is_err());
}
