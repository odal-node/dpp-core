//! ES256 hazards: algorithm confusion between two 64-byte signature formats,
//! JWK reading, and P-256 keys through the store. The vectors are in
//! `wycheproof/es256.rs`; the helpers they share are in `common.rs`.

use base64::Engine;
use serde_json::{Value, json};

use super::super::algorithm::KeyAlgorithm;
use super::super::error::{JwsError, SignatureLengthError};
use super::super::public_key::PublicKey;
use super::super::signer;
use super::super::verifier::verify_jws;
use super::common::{B64, rfc7515_key};
use crate::keystore::SigningMaterial;
use crate::test_support::{temp_path, temp_store};

// ── Algorithm confusion ───────────────────────────────────────────────────────

/// Four signature formats now coexist: Ed25519, and ECDSA over P-256, P-384 and
/// P-521. The key decides the algorithm, so a signature under any one key is
/// accepted only under that key and under one of its own identifiers, and each
/// substitution is refused by both verification paths.
#[test]
fn no_header_can_move_a_signature_to_the_other_algorithm() {
    let store = temp_store("es256-confusion", "ed");
    for (id, algorithm) in [
        ("p256", KeyAlgorithm::EcdsaP256),
        ("p384", KeyAlgorithm::EcdsaP384),
        ("p521", KeyAlgorithm::EcdsaP521),
    ] {
        store.generate_key_with(id, algorithm).expect("generate");
    }
    let ids = ["ed", "p256", "p384", "p521"];
    let keys: Vec<crate::keystore::KeyEntry> = ids
        .iter()
        .map(|id| store.load_key(id).expect("a key"))
        .collect();

    let token = |alg: &str, signer: &crate::keystore::KeyEntry| {
        let input = format!(
            "{}.{}",
            B64.encode(format!(r#"{{"alg":"{alg}"}}"#)),
            B64.encode(r#"{"claim":"value"}"#)
        );
        format!(
            "{input}.{}",
            B64.encode(
                signer
                    .sign(input.as_bytes())
                    .expect("an unrevoked key signs")
            )
        )
    };
    // The identifiers each algorithm answers to: Ed25519 has two.
    let names = |algorithm: KeyAlgorithm| match algorithm {
        KeyAlgorithm::Ed25519 => &["EdDSA", "Ed25519"][..],
        KeyAlgorithm::EcdsaP256 => &["ES256"][..],
        KeyAlgorithm::EcdsaP384 => &["ES384"][..],
        KeyAlgorithm::EcdsaP521 => &["ES512"][..],
    };

    let headers = ["EdDSA", "Ed25519", "ES256", "ES384", "ES512", "none"];
    for (id, signer) in ids.iter().zip(&keys) {
        for header in headers {
            let jws = token(header, signer);
            let own = names(signer.algorithm()).contains(&header);
            for (other_id, other) in ids.iter().zip(&keys) {
                // The controls are the cases where both hold: the signer's own
                // identifier, under the signer's own key. A signature whose length is
                // another algorithm's is malformed for this key (`Err`), and refused.
                let expected = own && other_id == id;
                assert_eq!(
                    verify_jws(&jws, &other.public_key()).is_ok(),
                    expected,
                    "{header} signed by {id}, verified under {other_id}"
                );
                assert_eq!(
                    signer::verify(&store, other_id, &jws).unwrap_or(false),
                    expected,
                    "{header} signed by {id}, verified under {other_id} in the store"
                );
            }
        }
    }
}

/// A JWK is read only into the verification path its type and curve name, with
/// full-length coordinates of a point on the curve.
#[test]
fn an_ec_jwk_must_be_p256_full_length_and_on_the_curve() {
    let good = rfc7515_key().jwk();
    assert!(PublicKey::from_jwk(&good).is_some());

    let mut wrong_curve = good.clone();
    wrong_curve["crv"] = json!("P-384");
    let mut okp_curve = good.clone();
    okp_curve["crv"] = json!("Ed25519");
    let mut okp_type = good.clone();
    okp_type["kty"] = json!("OKP");
    let mut short_x = good.clone();
    short_x["x"] = json!(B64.encode(&B64.decode(good["x"].as_str().unwrap()).unwrap()[1..]));
    let mut off_curve = good.clone();
    let mut y = B64.decode(good["y"].as_str().unwrap()).unwrap();
    y[31] ^= 1;
    off_curve["y"] = json!(B64.encode(&y));
    let mut no_y = good.clone();
    no_y.as_object_mut().unwrap().remove("y");

    for (name, jwk) in [
        ("wrong curve", wrong_curve),
        ("EC with an OKP curve", okp_curve),
        ("OKP with an EC curve", okp_type),
        ("shortened x", short_x),
        ("off the curve", off_curve),
        ("no y", no_y),
    ] {
        assert!(PublicKey::from_jwk(&jwk).is_none(), "{name}");
    }
}

/// A coordinate longer than its curve's is refused on every curve, `x` and `y` alike,
/// and before it is decoded: its encoded length is wrong, which is the whole refusal.
/// Each case is one octet too long, or a megabyte of `A`. An OKP key has only `x`.
#[test]
fn an_oversized_coordinate_is_refused_on_every_curve() {
    for algorithm in [
        KeyAlgorithm::Ed25519,
        KeyAlgorithm::EcdsaP256,
        KeyAlgorithm::EcdsaP384,
        KeyAlgorithm::EcdsaP521,
    ] {
        let good = SigningMaterial::generate(algorithm).public_key().jwk();
        assert!(PublicKey::from_jwk(&good).is_some(), "{algorithm:?}");

        let members: &[&str] = if good["kty"] == "EC" {
            &["x", "y"]
        } else {
            &["x"]
        };
        for member in members {
            let mut octets = B64
                .decode(good[*member].as_str().expect("a coordinate"))
                .expect("base64url");
            octets.push(0);
            let mut one_too_long = good.clone();
            one_too_long[*member] = json!(B64.encode(&octets));
            assert!(
                PublicKey::from_jwk(&one_too_long).is_none(),
                "{algorithm:?}: one octet too long in {member}"
            );

            let mut megabyte = good.clone();
            megabyte[*member] = json!("A".repeat(1 << 20));
            assert!(
                PublicKey::from_jwk(&megabyte).is_none(),
                "{algorithm:?}: a megabyte in {member}"
            );
        }
    }
}

/// A P-256 key given in SEC1 compressed form is the same key, held uncompressed,
/// with the same thumbprint.
#[test]
fn a_compressed_p256_key_is_the_same_key() {
    let key = SigningMaterial::generate(KeyAlgorithm::EcdsaP256).public_key();
    let uncompressed = key.as_bytes();
    let mut compressed = vec![2 + (uncompressed[64] & 1)];
    compressed.extend_from_slice(&uncompressed[1..33]);
    let again = PublicKey::from_bytes(KeyAlgorithm::EcdsaP256, &compressed).expect("compressed");
    assert_eq!(again, key);
    assert_eq!(again.thumbprint(), key.thumbprint());
}

// ── Keystore ──────────────────────────────────────────────────────────────────

/// A P-256 key survives the store: generated, persisted, reopened from disk,
/// loaded, and its signature verifies under the public key the store records.
/// The record names `ES256` on disk.
#[test]
fn a_p256_key_round_trips_through_the_encrypted_store() {
    let path = temp_path("es256-store");
    let generated = {
        let store = crate::keystore::KeyStore::open(&path, "pass").expect("open");
        store
            .generate_key_with("issuer", KeyAlgorithm::EcdsaP256)
            .expect("generate")
            .public_key()
    };
    let on_disk: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("read")).expect("json");
    assert_eq!(on_disk["keys"]["issuer"]["algorithm"], "ES256");
    assert_eq!(
        on_disk["keys"]["issuer"]["verifying_key_hex"]
            .as_str()
            .expect("hex")
            .len(),
        130,
        "65 bytes, SEC1 uncompressed"
    );

    let store = crate::keystore::KeyStore::open(&path, "pass").expect("reopen");
    let loaded = store.load_key("issuer").expect("load");
    assert_eq!(loaded.algorithm(), KeyAlgorithm::EcdsaP256);
    assert_eq!(loaded.public_key(), generated);
    let info = store.public_key("issuer").expect("info");
    assert_eq!(info.public_key().expect("valid"), generated);

    let jws = signer::sign(&store, "issuer", &json!({"a": 1}), "kid").expect("sign");
    verify_jws(&jws, &generated).expect("well-formed");
    let _ = std::fs::remove_file(path);
}

/// Rotation keeps the algorithm unless told otherwise, and a cross-algorithm
/// rotation leaves the old key archived under its own algorithm.
#[test]
fn rotation_keeps_or_changes_the_algorithm_as_asked() {
    let store = temp_store("es256-rotate", "issuer");
    assert_eq!(
        store.rotate_key("issuer").expect("rotate").algorithm(),
        KeyAlgorithm::Ed25519
    );
    let rotated = store
        .rotate_key_to("issuer", KeyAlgorithm::EcdsaP256)
        .expect("rotate to P-256");
    assert_eq!(rotated.algorithm(), KeyAlgorithm::EcdsaP256);
    assert_eq!(
        store.rotate_key("issuer").expect("rotate").algorithm(),
        KeyAlgorithm::EcdsaP256
    );
    let archived: Vec<KeyAlgorithm> = store
        .archived_public_keys("issuer")
        .iter()
        .map(|k| k.algorithm)
        .collect();
    assert_eq!(
        archived,
        [
            KeyAlgorithm::Ed25519,
            KeyAlgorithm::Ed25519,
            KeyAlgorithm::EcdsaP256
        ]
    );
}

/// A rotation that keeps the algorithm cannot undo a concurrent change of it. The
/// algorithm is checked under the lock the swap takes, and a rotation that finds it
/// changed generates again, so both serial orders of the two rotations leave the P-256
/// key installed. The race is timing-dependent, so the test catches a regression only
/// some of the time (about 1 run in 10 against the old code), and it never fails on
/// correct code.
#[test]
fn a_rotation_that_keeps_the_algorithm_cannot_undo_a_concurrent_change() {
    let store = temp_store("es256-race", "issuer");
    for round in 0..64 {
        store
            .rotate_key_to("issuer", KeyAlgorithm::Ed25519)
            .expect("reset to Ed25519");
        let barrier = std::sync::Barrier::new(2);
        std::thread::scope(|scope| {
            let keep = scope.spawn(|| {
                barrier.wait();
                store.rotate_key("issuer")
            });
            let change = scope.spawn(|| {
                barrier.wait();
                store.rotate_key_to("issuer", KeyAlgorithm::EcdsaP256)
            });
            keep.join().expect("thread").expect("rotate");
            change.join().expect("thread").expect("rotate to P-256");
        });
        assert_eq!(
            store.load_key("issuer").expect("load").algorithm(),
            KeyAlgorithm::EcdsaP256,
            "round {round}"
        );
    }
}

/// A signature of the wrong length is a typed error under either algorithm, so a caller
/// can tell it from a bad signature.
#[test]
fn a_wrong_length_signature_downcasts_to_its_error_under_both_algorithms() {
    for (algorithm, jose_alg, label, expected) in [
        (KeyAlgorithm::Ed25519, "EdDSA", "Ed25519", 64),
        (KeyAlgorithm::EcdsaP256, "ES256", "ES256", 64),
        (KeyAlgorithm::EcdsaP384, "ES384", "ES384", 96),
        (KeyAlgorithm::EcdsaP521, "ES512", "ES512", 132),
    ] {
        let key = SigningMaterial::generate(algorithm).public_key();
        let actual = expected - 1;
        let jws = format!(
            "{}.{}.{}",
            B64.encode(format!(r#"{{"alg":"{jose_alg}"}}"#)),
            B64.encode(r#"{"claim":"value"}"#),
            B64.encode(vec![0u8; actual]),
        );
        let err = verify_jws(&jws, &key).expect_err("a short signature is refused");
        assert_eq!(
            err,
            JwsError::SignatureLength(SignatureLengthError {
                algorithm,
                expected,
                actual,
            })
        );
        assert_eq!(
            err.to_string(),
            format!("{label} signature must be {expected} bytes")
        );
    }
}
