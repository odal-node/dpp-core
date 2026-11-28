//! ES256 against third-party vectors and an independent implementation.
//!
//! Sources, none of them written here:
//! - **Wycheproof** `ecdsa_secp256r1_sha256_p1363_test.json` (C2SP), the edge-case
//!   corpus for an IEEE P1363 (`R ‖ S`) verifier, which is the JOSE encoding.
//!   Vendored from the Wycheproof repository's test vectors, byte
//!   for byte the file at commit `878e5366008753df2064d40c49f8e2f50f9c6af7`
//!   (2026-05-12), the last commit that changed it. SHA-256
//!   `c60de693930e386c3a5472d08081623ef8504decc54b38ac01ec6b2a2575c986`. Licence:
//!   Apache License 2.0, the same licence as this crate, whose `LICENSE` ships with
//!   it. The upstream repository has no `NOTICE` file at that commit, so none is
//!   reproduced.
//! - **RFC 7515 Appendix A.3**, the IETF's ES256 JWS, reproduced as data.
//! - **RFC 6979 Appendix A.2.5**, deterministic ECDSA over P-256, reproduced as
//!   data.
//! - **`ring`**, a separate ECDSA implementation (BoringSSL-derived), signing and
//!   verifying in both directions.
//! - **The W3C VC-JOSE-COSE test suite's own fixtures**: its key files and the
//!   signed credentials its verification tests feed an implementation. Vendored
//!   from the suite's test inputs: `vm-p256.json`, `vm-ed25519.json`, and the four
//!   credential files beside `LICENSE.md`. Each
//!   file's git blob equals the repository's `main` as read on 2026-11-18. The last
//!   commit that changed `vm-p256.json` and `vm-ed25519.json` is
//!   `a3d7321314d4350667fa258293783a2fbccca4b6` (2024-12-13). The last commit that
//!   changed the four credential tokens is `002d73018cc0b27d11086fd06598c473c9142646`
//!   (2024-12-16).
//!   Licence: the W3C Software and Document License, reproduced in
//!   `fixtures/w3c-vc-jose-cose/LICENSE.md`.
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
use super::super::common::{B64, p256, rfc7515_key};
use crate::keystore::SigningMaterial;
use crate::test_support::temp_store;

// ── Wycheproof ────────────────────────────────────────────────────────────────

/// Every one of Wycheproof's 262 P-256/SHA-256 P1363 cases gets the verdict the
/// corpus records: 173 valid, 89 invalid. The invalid ones include `R` or `S`
/// of zero or at least `n`, wrong lengths, and arithmetic edge cases. Each
/// group's key is read from the corpus's own JWK and must equal the same key
/// read from its SEC1 bytes.
#[test]
fn every_wycheproof_p256_p1363_case_gets_its_recorded_verdict() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../fixtures/wycheproof_ecdsa_secp256r1_sha256_p1363_test.json"
    ))
    .expect("the corpus parses");
    assert_eq!(corpus["algorithm"], "ECDSA");

    let (mut valid, mut invalid, mut high_s_valid, mut jwk_groups) = (0, 0, 0, 0);
    for group in corpus["testGroups"].as_array().expect("groups") {
        assert_eq!(group["sha"], "SHA-256");
        let uncompressed =
            hex::decode(group["publicKey"]["uncompressed"].as_str().expect("hex")).expect("hex");
        let from_jwk = PublicKey::from_bytes(KeyAlgorithm::EcdsaP256, &uncompressed).expect("SEC1");
        // Most groups also give the key as a JWK (a few recent ones do not); where
        // they do, it must read as the same key.
        if !group["publicKeyJwk"].is_null() {
            assert_eq!(
                p256(&group["publicKeyJwk"]),
                from_jwk,
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
                from_jwk.verify(&message, &signature),
                expected,
                "tcId {} ({})",
                case["tcId"],
                case["comment"]
            );
            if expected {
                valid += 1;
                if signature.len() == 64 && is_high_s(&signature) {
                    high_s_valid += 1;
                }
            } else {
                invalid += 1;
            }
        }
    }
    assert_eq!((valid, invalid), (173, 89));
    assert_eq!(jwk_groups, 103, "112 groups, 9 without a JWK");
    // The corpus itself holds high-S signatures as valid ECDSA.
    assert!(
        high_s_valid > 0,
        "the corpus carries valid high-S signatures"
    );
}

// ── RFC 7515 A.3 ──────────────────────────────────────────────────────────────

const RFC7515_A3_JWS: &str = concat!(
    "eyJhbGciOiJFUzI1NiJ9",
    ".",
    "eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFt",
    "cGxlLmNvbS9pc19yb290Ijp0cnVlfQ",
    ".",
    "DtEhU3ljbEg8L38VWAfUAqOyKAM6-Xx-F4GawxaepmXFCgfTjDxw5djxLa8ISlSA",
    "pmWQxfKTUJqPP3-Kg6NU1Q",
);

/// The IETF's own ES256 JWS verifies, and its signature is **high-S**: a
/// verifier that refused high-S would refuse the RFC's example.
#[test]
fn the_rfc7515_es256_example_verifies_and_is_high_s() {
    verify_jws(RFC7515_A3_JWS, &rfc7515_key()).expect("well-formed");

    let signature = B64
        .decode(RFC7515_A3_JWS.rsplit('.').next().expect("signature"))
        .expect("base64url");
    assert!(is_high_s(&signature), "the RFC's S is above n/2");

    // Its low-S twin verifies too: ECDSA is malleable, so nothing may key on
    // signature bytes.
    let (input, _) = RFC7515_A3_JWS.rsplit_once('.').expect("three parts");
    let twin = format!("{input}.{}", B64.encode(negate_s(&signature)));
    verify_jws(&twin, &rfc7515_key()).expect("well-formed");

    // And a changed payload does not.
    let altered = RFC7515_A3_JWS.replacen("eyJpc3Mi", "eyJpc3Ni", 1);
    assert!(verify_jws(&altered, &rfc7515_key()).is_err());
}

// ── RFC 6979 A.2.5 ────────────────────────────────────────────────────────────

/// Deterministic signing reproduces RFC 6979's P-256/SHA-256 vectors. For
/// "sample" the RFC's `S` is high, so this crate writes `n − S`; for "test" it
/// is low and comes out byte for byte.
#[test]
fn signing_is_rfc6979_deterministic_and_low_s() {
    let secret = hex::decode("C9AFA9D845BA75166B5C215767B1D6934E50C3DB36E89B127B8A622B120F6721")
        .expect("hex");
    let key = SigningMaterial::from_secret(KeyAlgorithm::EcdsaP256, &secret).expect("a scalar");
    let public = key.public_key();
    assert_eq!(
        hex::encode_upper(&public.as_bytes()[1..]),
        "60FED4BA255A9D31C961EB74C6356D68C049B8923B61FA6CE669622E60F29FB6\
         7903FE1008B8BC99A41AE9E95628BC64F2F1B20C2D7E9F5177A3C294D4462299"
    );

    let sample = key.sign(b"sample");
    let rfc_sample = hex::decode(
        "EFD48B2AACB6A8FD1140DD9CD45E81D69D2C877B56AAF991C34D0EA84EAF3716\
         F7CB1C942D657C41D436C7A1B6E29F65F3E900DBB9AFF4064DC4AB2F843ACDA8",
    )
    .expect("hex");
    assert!(is_high_s(&rfc_sample));
    assert_eq!(sample, negate_s(&rfc_sample), "r, and n − s");
    assert_eq!(key.sign(b"sample"), sample, "deterministic");

    let test = key.sign(b"test");
    assert_eq!(
        hex::encode_upper(&test),
        "F1ABB023518351CD71D881567B1EA663ED3EFCF6C5132B354F28D3B0B7D38367\
         019F4113742A2B14BD25926B49C649155F267E60D3814B4C0CC84250E46F0083"
    );
}

// ── ring, both directions ─────────────────────────────────────────────────────

/// `ring` verifies the ES256 JWS this crate's signer produces, through the
/// keystore and the compact serialisation, for many payloads.
#[test]
fn ring_verifies_the_es256_jws_this_crate_signs() {
    let store = temp_store("es256-ring", "unused");
    let entry = store
        .generate_key_with("p256", KeyAlgorithm::EcdsaP256)
        .expect("generate");
    let public = entry.public_key();
    let ring_key = ring::signature::UnparsedPublicKey::new(
        &ring::signature::ECDSA_P256_SHA256_FIXED,
        public.as_bytes().to_vec(),
    );
    for i in 0..64 {
        let payload = json!({"i": i, "passport": format!("urn:example:{i}")});
        let jws = signer::sign(&store, "p256", &payload, "kid-1").expect("sign");
        let (input, signature) = jws.rsplit_once('.').expect("three parts");
        let header: Value = serde_json::from_slice(
            &B64.decode(jws.split('.').next().expect("header"))
                .expect("b64"),
        )
        .expect("json");
        assert_eq!(header["alg"], "ES256");
        let signature = B64.decode(signature).expect("base64url");
        assert_eq!(signature.len(), 64);
        assert!(!is_high_s(&signature), "this crate writes low-S");
        ring_key
            .verify(input.as_bytes(), &signature)
            .expect("ring accepts this crate's ES256 signature");
    }
}

/// This crate verifies what `ring` signs. `ring` draws a random nonce and does
/// not normalise, so about half its signatures are high-S; all must verify.
#[test]
fn this_crate_verifies_what_ring_signs_including_high_s() {
    use ring::signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair, KeyPair as _};
    let rng = ring::rand::SystemRandom::new();
    let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).expect("key");
    let pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref(), &rng)
        .expect("key");
    let key = PublicKey::from_bytes(KeyAlgorithm::EcdsaP256, pair.public_key().as_ref())
        .expect("ring's public key");

    let header = B64.encode(r#"{"alg":"ES256","typ":"vc+jwt"}"#);
    let mut high = 0;
    for i in 0..200 {
        let input = format!("{header}.{}", B64.encode(format!(r#"{{"i":{i}}}"#)));
        let signature = pair.sign(&rng, input.as_bytes()).expect("sign");
        if is_high_s(signature.as_ref()) {
            high += 1;
        }
        let jws = format!("{input}.{}", B64.encode(signature.as_ref()));
        assert!(verify_jws(&jws, &key).is_ok(), "case {i}");
    }
    assert!(high > 40, "ring produced {high} high-S signatures of 200");
}

/// This crate never emits high-S, over many keys and messages.
#[test]
fn this_crate_never_emits_high_s() {
    for k in 0..20 {
        let key = SigningMaterial::generate(KeyAlgorithm::EcdsaP256);
        for m in 0..50 {
            let signature = key.sign(format!("{k}:{m}").as_bytes());
            assert!(!is_high_s(&signature));
        }
    }
}

// ── The W3C VC-JOSE-COSE suite's own fixtures ─────────────────────────────────

fn vm(file: &str) -> Value {
    let text = match file {
        "p256" => include_str!("../../fixtures/w3c-vc-jose-cose/vm-p256.json"),
        "p384" => include_str!("../../fixtures/w3c-vc-jose-cose/vm-p384.json"),
        "p521" => include_str!("../../fixtures/w3c-vc-jose-cose/vm-p521.json"),
        "ed25519" => include_str!("../../fixtures/w3c-vc-jose-cose/vm-ed25519.json"),
        _ => unreachable!(),
    };
    serde_json::from_str(text).expect("a verification method")
}

/// The suite's key files carry a `kid` equal to the RFC 7638 thumbprint of the
/// public JWK. This crate's thumbprint matches all four: third-party vectors for
/// the EC members (`crv`, `kty`, `x`, `y`) on P-256, P-384 and P-521, and for OKP.
#[test]
fn the_suite_key_ids_are_this_crates_thumbprints() {
    for file in ["p256", "p384", "p521", "ed25519"] {
        let method = vm(file);
        let key = PublicKey::from_jwk(&method["publicKeyJwk"]).expect("a key");
        assert_eq!(
            key.thumbprint(),
            method["publicKeyJwk"]["kid"].as_str().expect("kid"),
            "{file}"
        );
    }
}

/// The suite's verification fixtures, under the keys its mapping pairs them
/// with: test 6 (ES256, must verify), test 8 (EdDSA, must verify), test 12 (a
/// bad signature, must not).
///
/// Test 9's fixture, "unknown extensions", carries **no `crit`**: it is an ES512
/// token paired with the Ed25519 key, so any verifier refuses it on the
/// algorithm alone. It cannot evidence `crit` handling.
#[test]
fn the_suite_verification_fixtures_get_the_expected_verdicts() {
    let p = PublicKey::from_jwk(&vm("p256")["publicKeyJwk"]).expect("p256");
    let ed = PublicKey::from_jwk(&vm("ed25519")["publicKeyJwk"]).expect("ed25519");
    let read = |text: &str| text.trim().to_owned();

    let test6 = read(include_str!(
        "../../fixtures/w3c-vc-jose-cose/credential-jose-minimal.txt"
    ));
    let test8 = read(include_str!(
        "../../fixtures/w3c-vc-jose-cose/credential-issuer-match-signed.txt"
    ));
    let test12 = read(include_str!(
        "../../fixtures/w3c-vc-jose-cose/credential-jose-bad-signature.txt"
    ));
    let test9 = read(include_str!(
        "../../fixtures/w3c-vc-jose-cose/credential-jose-unknown-extensions.txt"
    ));

    assert!(verify_jws(&test6, &p).is_ok(), "test 6");
    assert!(verify_jws(&test8, &ed).is_ok(), "test 8");
    assert!(verify_jws(&test12, &ed).is_err(), "test 12");

    let header: Value =
        serde_json::from_slice(&B64.decode(test9.split('.').next().unwrap()).expect("b64"))
            .expect("json");
    assert_eq!(header["alg"], "ES512");
    assert!(header.get("crit").is_none(), "test 9 carries no crit");
    assert!(verify_jws(&test9, &ed).is_err(), "test 9");
}

/// The order of the P-256 group, big-endian.
pub(super) const N: [u8; 32] =
    hex32("FFFFFFFF00000000FFFFFFFFFFFFFFFFBCE6FAADA7179E84F3B9CAC2FC632551");

/// `n / 2`, rounded down: an `S` above it is "high".
pub(super) const HALF_N: [u8; 32] =
    hex32("7FFFFFFF800000007FFFFFFFFFFFFFFFDE737D56D38BCF4279DCE5617E3192A8");

pub(super) const fn hex32(hex: &str) -> [u8; 32] {
    let bytes = hex.as_bytes();
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        out[i] = nibble(bytes[2 * i]) << 4 | nibble(bytes[2 * i + 1]);
        i += 1;
    }
    out
}

pub(super) const fn nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'A'..=b'F' => c - b'A' + 10,
        b'a'..=b'f' => c - b'a' + 10,
        _ => panic!("not hex"),
    }
}

pub(super) fn is_high_s(signature: &[u8]) -> bool {
    signature[32..] > HALF_N[..]
}

/// `n − s`, the other valid `S` for the same `R`.
pub(super) fn negate_s(signature: &[u8]) -> Vec<u8> {
    let mut out = signature[..32].to_vec();
    let s = &signature[32..];
    let mut negated = [0u8; 32];
    let mut borrow = 0i16;
    for i in (0..32).rev() {
        let mut d = i16::from(N[i]) - i16::from(s[i]) - borrow;
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
