//! ES384 (P-384, SHA-384) and ES512 (P-521, SHA-512) against third-party vectors and
//! an independent implementation.
//!
//! Sources, none of them written here:
//! - **Wycheproof** `ecdsa_secp384r1_sha384_p1363_test.json` and
//!   `ecdsa_secp521r1_sha512_p1363_test.json` (C2SP), the edge-case corpora for an
//!   IEEE P1363 (`R ‖ S`) verifier, which is the JOSE encoding. Vendored from
//!   Vendored from the Wycheproof repository's test vectors, byte for byte the
//!   files at commit `878e5366008753df2064d40c49f8e2f50f9c6af7`. Their git blobs are
//!   `be2d9c2856075274ce58b795a917304c706f79fb` (P-384) and
//!   `f0494e9f6cac4cfcb0a84da0357312b68952e00d` (P-521). SHA-256
//!   `e27344bf6daae75fa19663620883809f91c79fc4b4420eff9eae8036b34476be` and
//!   `d294cbe0485dff0a2c53f6b363103ddb81c444e7e5ee44df56549ea92b873840`. Licence:
//!   Apache License 2.0, the same licence as this crate, whose `LICENSE` ships with
//!   it. The upstream repository has no `NOTICE` file at that commit, so none is
//!   reproduced.
//! - **RFC 7515 Appendix A.4**, the IETF's ES512 JWS, reproduced as data.
//! - **RFC 6979 Appendix A.2.6** (P-384, SHA-384) and **A.2.7** (P-521, SHA-512),
//!   deterministic ECDSA, reproduced as data.
//! - **`ring`**, a separate ECDSA implementation (BoringSSL-derived), for P-384 in
//!   both directions. `ring` has no P-521 ECDSA, so ES512 has no `ring` check here:
//!   its vectors are the Wycheproof corpus, the RFC's own signature and the RFC's
//!   deterministic signatures.
//! - **The W3C VC-JOSE-COSE suite's key files** `vm-p384.json` and `vm-p521.json`,
//!   from the suite's test inputs. Each
//!   file's git blob equals the repository's `main` as read on 2026-11-18:
//!   `09c22e30af7d67a86ff3e00e5a1c1ff3cf7cdaf6` and
//!   `aac5c45870df4d4681b71f0df6efbb65cd7c0990`. Licence: the W3C Software and
//!   Document License, reproduced in `fixtures/w3c-vc-jose-cose/LICENSE.md`, which
//!   covers these two files too. The suite's P-384 verification
//!   fixture (its test 7) needs a presentation type, so it is not used here.
//!
//! The RFC 6979 and RFC 7515 data is reused as Code Components under the IETF Trust's
//! Legal Provisions Relating to IETF Documents. In place of the licence text, each
//! RFC's data carries the notification of Section 6.d of those provisions, with that
//! RFC's year.
//!
//! For RFC 6979:
//!
//! > Copyright (c) 2013 IETF Trust and the persons identified as authors of the
//! > code. All rights reserved.
//! >
//! > Redistribution and use in source and binary forms, with or without
//! > modification, is permitted pursuant to, and subject to the license terms
//! > contained in, the Revised BSD License set forth in Section 4.c of the IETF
//! > Trust's Legal Provisions Relating to IETF Documents
//! > (<https://trustee.ietf.org/license-info>).
//!
//! For RFC 7515:
//!
//! > Copyright (c) 2015 IETF Trust and the persons identified as authors of the
//! > code. All rights reserved.
//! >
//! > Redistribution and use in source and binary forms, with or without
//! > modification, is permitted pursuant to, and subject to the license terms
//! > contained in, the Revised BSD License set forth in Section 4.c of the IETF
//! > Trust's Legal Provisions Relating to IETF Documents
//! > (<https://trustee.ietf.org/license-info>).

use base64::Engine;
use serde_json::{Value, json};

use super::super::super::algorithm::KeyAlgorithm;
use super::super::super::public_key::PublicKey;
use super::super::super::signer;
use super::super::super::verifier::verify_jws;
use super::super::common::{
    B64, HALF_P384, HALF_P521, RFC6979_P384_UX, RFC6979_P384_UY, hex_bytes, is_high_s,
    rfc7515_p521_key,
};
use crate::keystore::SigningMaterial;
use crate::test_support::temp_store;

/// The affine coordinates of RFC 6979 clause A.2.7's P-521 public key.
const RFC6979_P521_UX: &str = "01894550D0785932E00EAA23B694F213F8C3121F86DC97A04E5A7167DB4E5BCD371123D46E45DB6B5D5370A7F20FB633155D38FFA16D2BD761DCAC474B9A2F5023A4";
const RFC6979_P521_UY: &str = "00493101C962CD4D2FDDF782285E64584139C2F91B47F87FF82354D6630F746A28A0DB25741B5B34A828008B22ACC23F924FAAFBD4D33F81EA66956DFEAA2BFDFCF5";

// ── Wycheproof ────────────────────────────────────────────────────────────────

/// Every one of Wycheproof's 280 P-384/SHA-384 P1363 cases gets the verdict the
/// corpus records: 193 valid, 87 invalid. Each group's key is read from its SEC1
/// bytes, and where the group also gives a JWK, the JWK must read as the same key.
#[test]
fn every_wycheproof_p384_p1363_case_gets_its_recorded_verdict() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../fixtures/wycheproof_ecdsa_secp384r1_sha384_p1363_test.json"
    ))
    .expect("the corpus parses");
    assert_eq!(corpus["algorithm"], "ECDSA");

    let (mut valid, mut invalid, mut high_s_valid, mut jwk_groups) = (0, 0, 0, 0);
    for group in corpus["testGroups"].as_array().expect("groups") {
        assert_eq!(group["sha"], "SHA-384");
        let uncompressed =
            hex::decode(group["publicKey"]["uncompressed"].as_str().expect("hex")).expect("hex");
        let key = PublicKey::from_bytes(KeyAlgorithm::EcdsaP384, &uncompressed).expect("SEC1");
        if group.get("publicKeyJwk").is_some_and(|jwk| !jwk.is_null()) {
            assert_eq!(
                PublicKey::from_jwk(&group["publicKeyJwk"]).expect("a JWK"),
                key,
                "the JWK and the SEC1 form are one key"
            );
            jwk_groups += 1;
        }
        for case in group["tests"].as_array().expect("tests") {
            let message = hex::decode(case["msg"].as_str().expect("msg")).expect("hex");
            let signature = hex::decode(case["sig"].as_str().expect("sig")).expect("hex");
            let expected = match case["result"].as_str() {
                Some("valid") => true,
                Some("invalid") => false,
                other => panic!("tcId {}: unexpected result {other:?}", case["tcId"]),
            };
            assert_eq!(
                key.verify(&message, &signature),
                expected,
                "tcId {} ({})",
                case["tcId"],
                case["comment"]
            );
            if expected {
                valid += 1;
                if signature.len() == 96 && is_high_s(&HALF_P384, &signature) {
                    high_s_valid += 1;
                }
            } else {
                invalid += 1;
            }
        }
    }
    assert_eq!((valid, invalid), (193, 87));
    assert_eq!(jwk_groups, 95, "104 groups, 9 without a JWK");
    assert!(
        high_s_valid > 0,
        "the corpus carries valid high-S signatures"
    );
}

/// Every one of Wycheproof's 318 P-521/SHA-512 P1363 cases gets the verdict the
/// corpus records: 231 valid, 87 invalid. The key and JWK rules are those of the P-384
/// corpus above.
#[test]
fn every_wycheproof_p521_p1363_case_gets_its_recorded_verdict() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../fixtures/wycheproof_ecdsa_secp521r1_sha512_p1363_test.json"
    ))
    .expect("the corpus parses");
    assert_eq!(corpus["algorithm"], "ECDSA");

    let (mut valid, mut invalid, mut high_s_valid, mut jwk_groups) = (0, 0, 0, 0);
    for group in corpus["testGroups"].as_array().expect("groups") {
        assert_eq!(group["sha"], "SHA-512");
        let uncompressed =
            hex::decode(group["publicKey"]["uncompressed"].as_str().expect("hex")).expect("hex");
        let key = PublicKey::from_bytes(KeyAlgorithm::EcdsaP521, &uncompressed).expect("SEC1");
        if group.get("publicKeyJwk").is_some_and(|jwk| !jwk.is_null()) {
            assert_eq!(
                PublicKey::from_jwk(&group["publicKeyJwk"]).expect("a JWK"),
                key,
                "the JWK and the SEC1 form are one key"
            );
            jwk_groups += 1;
        }
        for case in group["tests"].as_array().expect("tests") {
            let message = hex::decode(case["msg"].as_str().expect("msg")).expect("hex");
            let signature = hex::decode(case["sig"].as_str().expect("sig")).expect("hex");
            let expected = match case["result"].as_str() {
                Some("valid") => true,
                Some("invalid") => false,
                other => panic!("tcId {}: unexpected result {other:?}", case["tcId"]),
            };
            assert_eq!(
                key.verify(&message, &signature),
                expected,
                "tcId {} ({})",
                case["tcId"],
                case["comment"]
            );
            if expected {
                valid += 1;
                if signature.len() == 132 && is_high_s(&HALF_P521, &signature) {
                    high_s_valid += 1;
                }
            } else {
                invalid += 1;
            }
        }
    }
    assert_eq!((valid, invalid), (231, 87));
    assert_eq!(jwk_groups, 98, "107 groups, 9 without a JWK");
    assert!(
        high_s_valid > 0,
        "the corpus carries valid high-S signatures"
    );
}

// ── RFC 7515 Appendix A.4 ─────────────────────────────────────────────────────

const RFC7515_A4_JWS: &str = concat!(
    "eyJhbGciOiJFUzUxMiJ9",
    ".",
    "UGF5bG9hZA",
    ".",
    "AdwMgeerwtHoh-l192l60hp9wAHZFVJbLfD_UxMi70cwnZOYaRI1bKPWROc-mZZq",
    "wqT2SI-KGDKB34XO0aw_7XdtAG8GaSwFKdCAPZgoXD2YBJZCPEX3xKpRwcdOO8Kp",
    "EHwJjyqOgzDO7iKvU8vcnwNrmxYbSW9ERBXukOXolLzeO_Jn",
);

/// The IETF's ES512 JWS verifies under the RFC's P-521 key. Its signature's `S` is
/// low, so this example does not reach the high-S path that the ES256 example does;
/// the high-S cases are the Wycheproof corpus and the RFC 6979 vectors below. A
/// changed payload does not verify.
#[test]
fn the_rfc7515_es512_example_verifies_and_an_altered_payload_does_not() {
    let key = rfc7515_p521_key();
    assert!(verify_jws(RFC7515_A4_JWS, &key).expect("well-formed"));

    let signature = B64
        .decode(RFC7515_A4_JWS.rsplit('.').next().expect("signature"))
        .expect("base64url");
    assert_eq!(signature.len(), 132);
    assert!(
        !is_high_s(&HALF_P521, &signature),
        "the RFC's S is at most n/2"
    );

    let altered = RFC7515_A4_JWS.replacen("UGF5bG9hZA", "UGF5bG9hZB", 1);
    assert!(!verify_jws(&altered, &key).expect("well-formed"));
}

// ── RFC 6979 Appendix A.2.6 and A.2.7 ─────────────────────────────────────────

/// Deterministic signing reproduces RFC 6979 clause A.2.6's P-384/SHA-384 vectors.
/// Both of the RFC's signatures for these messages have a high `S`, so this crate
/// writes `n − S` for each, and the same message always gets the same signature.
#[test]
fn p384_signing_is_rfc6979_deterministic_and_low_s() {
    let secret = hex::decode(
        "6B9D3DAD2E1B8C1C05B19875B6659F4DE23C3B667BF297BA9AA47740787137D8\
         96D5724E4C70A825F872C9EA60D2EDF5",
    )
    .expect("hex");
    let key = SigningMaterial::from_secret(KeyAlgorithm::EcdsaP384, &secret).expect("a scalar");
    assert_eq!(
        hex::encode_upper(&key.public_key().as_bytes()[1..]),
        format!("{RFC6979_P384_UX}{RFC6979_P384_UY}")
    );

    let rfc_sample = hex::decode(concat!(
        "94EDBB92A5ECB8AAD4736E56C691916B3F88140666CE9FA73D64C4EA95AD133C",
        "81A648152E44ACF96E36DD1E80FABE46",
        "99EF4AEB15F178CEA1FE40DB2603138F130E740A19624526203B6351D0A3A94F",
        "A329C145786E679E7B82C71A38628AC8",
    ))
    .expect("hex");
    assert!(is_high_s(&HALF_P384, &rfc_sample));
    let sample = key.sign(b"sample");
    assert_eq!(sample, negate_s(&ORDER_P384, &rfc_sample), "r, and n − s");
    assert_eq!(key.sign(b"sample"), sample, "deterministic");

    let rfc_test = hex::decode(concat!(
        "8203B63D3C853E8D77227FB377BCF7B7B772E97892A80F36AB775D509D7A5FEB",
        "0542A7F0812998DA8F1DD3CA3CF023DB",
        "DDD0760448D42D8A43AF45AF836FCE4DE8BE06B485E9B61B827C2F13173923E0",
        "6A739F040649A667BF3B828246BAA5A5",
    ))
    .expect("hex");
    assert!(is_high_s(&HALF_P384, &rfc_test));
    assert_eq!(key.sign(b"test"), negate_s(&ORDER_P384, &rfc_test), "n − S");
}

/// Deterministic signing reproduces RFC 6979 clause A.2.7's P-521/SHA-512 vectors.
/// The RFC's "sample" signature has a low `S`, which this crate writes as it is; its
/// "test" signature has a high `S`, which this crate writes as `n − S`.
#[test]
fn p521_signing_is_rfc6979_deterministic_and_low_s() {
    let secret = hex::decode(
        "00FAD06DAA62BA3B25D2FB40133DA757205DE67F5BB0018FEE8C86E1B68C7E75C\
         AA896EB32F1F47C70855836A6D16FCC1466F6D8FBEC67DB89EC0C08B0E996B83538",
    )
    .expect("hex");
    let key = SigningMaterial::from_secret(KeyAlgorithm::EcdsaP521, &secret).expect("a scalar");
    assert_eq!(
        hex::encode_upper(&key.public_key().as_bytes()[1..]),
        format!("{RFC6979_P521_UX}{RFC6979_P521_UY}")
    );

    let rfc_sample = hex::decode(concat!(
        "00C328FAFCBD79DD77850370C46325D987CB525569FB63C5D3BC53950E6D4C5F",
        "174E25A1EE9017B5D450606ADD152B534931D7D4E8455CC91F9B15BF05EC36E377FA",
        "00617CCE7CF5064806C467F678D3B4080D6F1CC50AF26CA209417308281B68AF2",
        "82623EAA63E5B5C0723D8B8C37FF0777B1A20F8CCB1DCCC43997F1EE0E44DA4A67A",
    ))
    .expect("hex");
    assert!(!is_high_s(&HALF_P521, &rfc_sample));
    assert_eq!(key.sign(b"sample"), rfc_sample, "as the RFC has it");
    assert_eq!(key.sign(b"sample"), rfc_sample, "deterministic");

    let rfc_test = hex::decode(concat!(
        "013E99020ABF5CEE7525D16B69B229652AB6BDF2AFFCAEF38773B4B7D08725F10",
        "CDB93482FDCC54EDCEE91ECA4166B2A7C6265EF0CE2BD7051B7CEF945BABD47EE6D",
        "01FBD0013C674AA79CB39849527916CE301C66EA7CE8B80682786AD60F98F7E78",
        "A19CA69EFF5C57400E3B3A0AD66CE0978214D13BAF4E9AC60752F7B155E2DE4DCE3",
    ))
    .expect("hex");
    assert!(is_high_s(&HALF_P521, &rfc_test));
    assert_eq!(key.sign(b"test"), negate_s(&ORDER_P521, &rfc_test), "n − S");
}

// ── ring, both directions, for P-384 ──────────────────────────────────────────

/// `ring` verifies the ES384 JWS this crate's signer produces, through the keystore
/// and the compact serialisation, for many payloads.
#[test]
fn ring_verifies_the_es384_jws_this_crate_signs() {
    let store = temp_store("es384-ring", "unused");
    let entry = store
        .generate_key_with("p384", KeyAlgorithm::EcdsaP384)
        .expect("generate");
    let public = entry.public_key();
    let ring_key = ring::signature::UnparsedPublicKey::new(
        &ring::signature::ECDSA_P384_SHA384_FIXED,
        public.as_bytes().to_vec(),
    );
    for i in 0..64 {
        let payload = json!({"i": i, "passport": format!("urn:example:{i}")});
        let jws = signer::sign(&store, "p384", &payload, "kid-1").expect("sign");
        let (input, signature) = jws.rsplit_once('.').expect("three parts");
        let header: Value = serde_json::from_slice(
            &B64.decode(jws.split('.').next().expect("header"))
                .expect("b64"),
        )
        .expect("json");
        assert_eq!(header["alg"], "ES384");
        let signature = B64.decode(signature).expect("base64url");
        assert_eq!(signature.len(), 96);
        assert!(
            !is_high_s(&HALF_P384, &signature),
            "this crate writes low-S"
        );
        ring_key
            .verify(input.as_bytes(), &signature)
            .expect("ring accepts this crate's ES384 signature");
    }
}

/// This crate verifies what `ring` signs, including the high-S signatures `ring`
/// produces: `ring` draws a random nonce and does not normalise, so about half of its
/// P-384 signatures are high-S. All of them must verify.
#[test]
fn this_crate_verifies_what_ring_signs_including_high_s_p384() {
    use ring::signature::{ECDSA_P384_SHA384_FIXED_SIGNING, EcdsaKeyPair, KeyPair as _};
    let rng = ring::rand::SystemRandom::new();
    let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P384_SHA384_FIXED_SIGNING, &rng).expect("key");
    let pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P384_SHA384_FIXED_SIGNING, pkcs8.as_ref(), &rng)
        .expect("key");
    let key = PublicKey::from_bytes(KeyAlgorithm::EcdsaP384, pair.public_key().as_ref())
        .expect("ring's public key");

    let header = B64.encode(r#"{"alg":"ES384","typ":"vc+jwt"}"#);
    let mut high = 0;
    for i in 0..200 {
        let input = format!("{header}.{}", B64.encode(format!(r#"{{"i":{i}}}"#)));
        let signature = pair.sign(&rng, input.as_bytes()).expect("sign");
        if is_high_s(&HALF_P384, signature.as_ref()) {
            high += 1;
        }
        let jws = format!("{input}.{}", B64.encode(signature.as_ref()));
        assert!(verify_jws(&jws, &key).expect("well-formed"), "case {i}");
    }
    assert!(high > 40, "ring produced {high} high-S signatures of 200");
}

/// The group order `n` of P-384, big-endian.
pub(super) const ORDER_P384: [u8; 48] = hex_bytes(
    "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFC7634D81F4372DDF581A0DB248B0A77AECEC196ACCC52973",
);

/// The group order `n` of P-521, big-endian, in its 66 octets.
pub(super) const ORDER_P521: [u8; 66] = hex_bytes(
    "01FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFA51868783BF2F966B7FCC0148F709A5D03BB5C9B8899C47AEBB6FB71E91386409",
);

/// `n − s`, the other valid `S` for the same `R`, over the `N`-octet order.
pub(super) fn negate_s<const N: usize>(order: &[u8; N], signature: &[u8]) -> Vec<u8> {
    let mut out = signature[..N].to_vec();
    let s = &signature[N..];
    let mut negated = [0u8; N];
    let mut borrow = 0i16;
    for i in (0..N).rev() {
        let mut d = i16::from(order[i]) - i16::from(s[i]) - borrow;
        borrow = 0;
        if d < 0 {
            d += 256;
            borrow = 1;
        }
        negated[i] = u8::try_from(d).expect("a byte");
    }
    out.extend_from_slice(&negated);
    out
}
