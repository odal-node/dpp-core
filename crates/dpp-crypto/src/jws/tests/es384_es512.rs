//! ES384 (P-384 with SHA-384) and ES512 (P-521 with SHA-512): the checks that the
//! ES256 tests make for P-256, made for the two larger curves beside it. The
//! third-party vectors are in `wycheproof/es384_es512.rs`.

use base64::Engine;
use serde_json::{Value, json};

use super::super::algorithm::KeyAlgorithm;
use super::super::public_key::PublicKey;
use super::common::{B64, HALF_P384, HALF_P521, is_high_s};
use crate::keystore::SigningMaterial;

/// Whether `signature` is high-S under `algorithm`, which must be P-384 or P-521.
fn high_s(algorithm: KeyAlgorithm, signature: &[u8]) -> bool {
    match algorithm {
        KeyAlgorithm::EcdsaP384 => is_high_s(&HALF_P384, signature),
        KeyAlgorithm::EcdsaP521 => is_high_s(&HALF_P521, signature),
        other => panic!("{other:?} is not one of the two larger curves"),
    }
}

// ── Keys: the JWK and the SEC1 form ──────────────────────────────────────────

/// An EC JWK for P-384 or P-521 is read only with full-length coordinates of a point
/// on its curve, and its curve name decides the algorithm.
#[test]
fn an_ec_jwk_must_be_full_length_and_on_its_curve_for_p384_and_p521() {
    for (algorithm, crv, field_len) in [
        (KeyAlgorithm::EcdsaP384, "P-384", 48),
        (KeyAlgorithm::EcdsaP521, "P-521", 66),
    ] {
        let good = SigningMaterial::generate(algorithm).public_key().jwk();
        assert_eq!(good["crv"], crv);
        assert_eq!(
            PublicKey::from_jwk(&good).expect("a key").algorithm(),
            algorithm
        );

        let x = B64
            .decode(good["x"].as_str().expect("x"))
            .expect("base64url");
        let y = B64
            .decode(good["y"].as_str().expect("y"))
            .expect("base64url");
        assert_eq!(x.len(), field_len, "{crv}: the full size of a coordinate");

        // The other curve's name on these coordinates: the length gives it away.
        let other = if crv == "P-384" { "P-521" } else { "P-384" };
        let mut wrong_curve = good.clone();
        wrong_curve["crv"] = json!(other);
        let mut short_x = good.clone();
        short_x["x"] = json!(B64.encode(&x[1..]));
        let mut off_curve = good.clone();
        let mut flipped = y.clone();
        flipped[field_len - 1] ^= 1;
        off_curve["y"] = json!(B64.encode(&flipped));
        let mut no_y = good.clone();
        no_y.as_object_mut().expect("an object").remove("y");
        let mut okp = good.clone();
        okp["kty"] = json!("OKP");

        for (name, jwk) in [
            ("labelled with the other curve", wrong_curve),
            ("shortened x", short_x),
            ("off the curve", off_curve),
            ("no y", no_y),
            ("OKP with an EC curve", okp),
        ] {
            assert!(PublicKey::from_jwk(&jwk).is_none(), "{crv}: {name}");
        }
    }
}

/// A P-384 or P-521 key given in SEC1 compressed form is the same key, held
/// uncompressed, with the same thumbprint.
#[test]
fn a_compressed_p384_or_p521_key_is_the_same_key() {
    for algorithm in [KeyAlgorithm::EcdsaP384, KeyAlgorithm::EcdsaP521] {
        let key = SigningMaterial::generate(algorithm).public_key();
        let uncompressed = key.as_bytes();
        let field_len = (uncompressed.len() - 1) / 2;
        // The tag is 2 for an even `y`, 3 for an odd one: the parity of its last octet.
        let mut compressed = vec![2 + (uncompressed[uncompressed.len() - 1] & 1)];
        compressed.extend_from_slice(&uncompressed[1..1 + field_len]);
        let again = PublicKey::from_bytes(algorithm, &compressed).expect("compressed");
        assert_eq!(again, key, "{algorithm:?}");
        assert_eq!(again.thumbprint(), key.thumbprint(), "{algorithm:?}");
    }
}

// ── Keystore ──────────────────────────────────────────────────────────────────

/// A P-384 key and a P-521 key survive the store: generated, persisted, reopened,
/// loaded and used, under the identifiers `ES384` and `ES512` on disk. Rotation keeps
/// a record's algorithm unless it is told to change it, and archives the old key
/// under its own algorithm.
#[test]
fn p384_and_p521_keys_round_trip_through_the_encrypted_store() {
    let path = crate::test_support::temp_path("es384-store");
    let (p384, p521) = {
        let store = crate::keystore::KeyStore::open(&path, "pass").expect("open");
        let p384 = store
            .generate_key_with("p384", KeyAlgorithm::EcdsaP384)
            .expect("generate")
            .public_key();
        let p521 = store
            .generate_key_with("p521", KeyAlgorithm::EcdsaP521)
            .expect("generate")
            .public_key();
        (p384, p521)
    };
    let on_disk: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("read")).expect("json");
    assert_eq!(on_disk["keys"]["p384"]["algorithm"], "ES384");
    assert_eq!(on_disk["keys"]["p521"]["algorithm"], "ES512");
    assert_eq!(
        on_disk["keys"]["p384"]["verifying_key_hex"]
            .as_str()
            .expect("hex")
            .len(),
        194,
        "97 bytes, SEC1 uncompressed"
    );
    assert_eq!(
        on_disk["keys"]["p521"]["verifying_key_hex"]
            .as_str()
            .expect("hex")
            .len(),
        266,
        "133 bytes, SEC1 uncompressed"
    );

    let store = crate::keystore::KeyStore::open(&path, "pass").expect("reopen");
    for (id, algorithm, public) in [
        ("p384", KeyAlgorithm::EcdsaP384, &p384),
        ("p521", KeyAlgorithm::EcdsaP521, &p521),
    ] {
        let loaded = store.load_key(id).expect("load");
        assert_eq!(loaded.algorithm(), algorithm);
        assert_eq!(&loaded.public_key(), public, "{id}");
        let info = store.public_key(id).expect("info");
        assert_eq!(&info.public_key().expect("valid"), public, "{id}");
        let jws = crate::jws::signer::sign(&store, id, &json!({"a": 1}), "kid").expect("sign");
        assert!(super::super::verifier::verify_jws(&jws, public).expect("well-formed"));
    }

    assert_eq!(
        store.rotate_key("p384").expect("rotate").algorithm(),
        KeyAlgorithm::EcdsaP384
    );
    let rotated = store
        .rotate_key_to("p384", KeyAlgorithm::EcdsaP521)
        .expect("rotate to P-521");
    assert_eq!(rotated.algorithm(), KeyAlgorithm::EcdsaP521);
    let archived: Vec<KeyAlgorithm> = store
        .archived_public_keys("p384")
        .iter()
        .map(|k| k.algorithm)
        .collect();
    assert_eq!(archived, [KeyAlgorithm::EcdsaP384, KeyAlgorithm::EcdsaP384]);
    let _ = std::fs::remove_file(path);
}

// ── Signing ───────────────────────────────────────────────────────────────────

/// This crate never emits high-S for P-384 or P-521, over the same sample size as
/// P-256: 20 keys and 50 messages each.
#[test]
fn this_crate_never_emits_high_s_for_p384_or_p521() {
    for k in 0..20 {
        for algorithm in [KeyAlgorithm::EcdsaP384, KeyAlgorithm::EcdsaP521] {
            let key = SigningMaterial::generate(algorithm);
            for m in 0..50 {
                let signature = key.sign(format!("{k}:{m}").as_bytes());
                assert!(!high_s(algorithm, &signature), "{algorithm:?}");
            }
        }
    }
}
