//! JAdES-B-B construction tests.
//!
//! Every assertion here is traceable to a clause of ETSI TS 119 182-1 V1.2.1,
//! and the clause is named. A test that pins a format against nothing but our
//! own reading of it is a test that agrees with us; naming the clause is what
//! lets the next reader check the reading rather than the code.

use base64::Engine;
use ed25519_dalek::{Signer, SigningKey, Verifier};
use serde_json::Value;

use super::*;

const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// A stand-in for a DER certificate. The bytes are never parsed by this module
/// — `x5t#S256` digests them and `x5c` carries them — so a fixed blob is enough
/// to exercise the format without pulling in a certificate generator.
const FAKE_DER: &[u8] = b"\x30\x82\x01\x0a-not-a-real-certificate-";

fn header() -> JadesHeader {
    JadesHeader {
        alg: "EdDSA".into(),
        iat: 1_770_000_000,
        certificate: CertificateRef::thumbprint_of_der(FAKE_DER),
        content_type: None,
    }
}

fn decode_header(compact: &str) -> Value {
    let seg = compact.split('.').next().expect("has a header segment");
    serde_json::from_slice(&B64.decode(seg).expect("header is base64url")).expect("header is JSON")
}

/// The header carries `alg`, a certificate reference and `iat` — and nothing
/// the standard does not ask for at B-B.
///
/// Table 1: `alg` shall be present; the claimed-signing-time service shall be
/// provided; the signing-certificate-reference service has cardinality 1.
#[test]
fn a_b_b_header_carries_exactly_what_the_standard_requires() {
    let prepared = prepare(&header(), b"payload").expect("prepares");
    let signed = prepared.assemble(&[0u8; 64]);
    let h = decode_header(signed.as_str());

    assert_eq!(h["alg"], "EdDSA");
    assert!(
        h.get("x5t#S256").is_some(),
        "clause 5.1.7 requires a reference"
    );
    assert!(h.get("iat").is_some(), "clause 5.1.11 requires iat");

    // Sorted, because `serde_json::Map` is a `BTreeMap` without the
    // `preserve_order` feature. JWS places no constraint on header key order,
    // and the property that matters — that the assembled bytes are the signed
    // bytes — comes from retaining the encoded segment, not from the ordering.
    let mut keys: Vec<&str> = h
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec!["alg", "iat", "x5t#S256"],
        "no parameter the standard does not ask for at B-B"
    );
}

/// `iat` is an integer number of seconds.
///
/// Clause 5.1.11: the value *"shall be an integer number"* and *"shall not
/// contain fractions of seconds"*. A float here would be a conformance defect
/// that no signature check would ever catch.
#[test]
fn iat_is_a_whole_number_of_seconds() {
    let prepared = prepare(&header(), b"payload").expect("prepares");
    let h = decode_header(prepared.assemble(&[0u8; 64]).as_str());
    let iat = &h["iat"];
    assert!(iat.is_i64(), "iat must be an integer, got {iat}");
    assert_eq!(iat.as_i64(), Some(1_770_000_000));
    assert!(
        !serde_json::to_string(iat)
            .expect("serialises")
            .contains('.'),
        "iat must carry no fractional part"
    );
}

/// No `crit`, and no `sigD`, for an attached payload.
///
/// Clause 5.2.8.1: `sigD` *"shall not appear in JAdES signatures whose JWS
/// Payload is attached"*. Clause 5.1.9: `crit` is required only when `sigD` is
/// present — V1.2.1 **suppressed** V1.1.1's blanket requirement precisely so
/// that a signature without `sigD` stays processable by a plain JWS library.
///
/// This test is the one that would fail if someone "improved" the header by
/// adding `crit` from memory of the older version.
#[test]
fn an_attached_payload_emits_neither_sigd_nor_crit() {
    let prepared = prepare(&header(), b"payload").expect("prepares");
    let h = decode_header(prepared.assemble(&[0u8; 64]).as_str());
    assert!(
        h.get("sigD").is_none(),
        "clause 5.2.8.1 forbids sigD when attached"
    );
    assert!(
        h.get("crit").is_none(),
        "clause 5.1.9 requires crit only alongside sigD; V1.2.1 NOTE 1 explains why"
    );
}

/// The result is a plain RFC 7515 compact JWS, verifiable as one.
///
/// The payoff of the clause above: one artefact serves a verifier that
/// understands AdES and one that only understands JWS. Here the whole
/// round-trip runs through an ordinary Ed25519 verify.
#[test]
fn the_output_verifies_as_an_ordinary_jws() {
    let key = SigningKey::generate(&mut crate::os_rng());
    let payload = br#"{"passportId":"abc","sector":"battery"}"#;

    let prepared = prepare(&header(), payload).expect("prepares");
    let signature = key.sign(prepared.signing_input());
    let compact = prepared.assemble(&signature.to_bytes()).into_string();

    let parts: Vec<&str> = compact.split('.').collect();
    assert_eq!(parts.len(), 3, "compact serialisation is three segments");

    // Verify exactly as a plain JWS consumer would: recompute the signing input
    // from the wire, do not trust what we kept in memory.
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let sig_bytes: [u8; 64] = B64
        .decode(parts[2])
        .expect("signature is base64url")
        .try_into()
        .expect("Ed25519 signature is 64 bytes");
    key.verifying_key()
        .verify(
            signing_input.as_bytes(),
            &ed25519_dalek::Signature::from_bytes(&sig_bytes),
        )
        .expect("a JAdES-B-B signature verifies as a plain JWS");

    assert_eq!(
        B64.decode(parts[1]).expect("payload is base64url"),
        payload,
        "the payload round-trips unchanged"
    );
}

/// Tampering with the payload breaks the signature.
///
/// Content binding is the whole point; a format test that never checks it has
/// only tested the packaging.
#[test]
fn a_tampered_payload_no_longer_verifies() {
    let key = SigningKey::generate(&mut crate::os_rng());
    let prepared = prepare(&header(), b"original").expect("prepares");
    let signature = key.sign(prepared.signing_input());
    let compact = prepared.assemble(&signature.to_bytes()).into_string();

    let parts: Vec<&str> = compact.split('.').collect();
    let tampered = format!("{}.{}", parts[0], B64.encode(b"substituted"));
    let sig_bytes: [u8; 64] = B64.decode(parts[2]).expect("b64").try_into().expect("64");

    assert!(
        key.verifying_key()
            .verify(
                tampered.as_bytes(),
                &ed25519_dalek::Signature::from_bytes(&sig_bytes)
            )
            .is_err(),
        "a substituted payload must not verify"
    );
}

/// A signature over the *digest* of the signing input is the remote-signing
/// hand-off, and it commits to the same bytes.
///
/// This is the operation a provider's "sign this hash" endpoint performs, and
/// the test exists to pin that the digest is taken over the signing input —
/// not over the payload, which is the easy and silent mistake.
#[test]
fn the_digest_helper_commits_to_the_signing_input() {
    use sha2::{Digest, Sha256};
    let prepared = prepare(&header(), b"payload").expect("prepares");
    let expected: [u8; 32] = Sha256::digest(prepared.signing_input()).into();
    assert_eq!(prepared.signing_input_sha256(), expected);

    // And explicitly *not* a digest of the payload alone, which would sign
    // something that omits the header — including the certificate reference.
    let payload_only: [u8; 32] = Sha256::digest(b"payload").into();
    assert_ne!(
        prepared.signing_input_sha256(),
        payload_only,
        "the signature must cover the header, not only the payload"
    );
}

/// A missing certificate reference is refused, not silently omitted.
///
/// Clause 5.1.7 makes one of `x5t#S256`/`x5c`/`sigX5ts`/`x5t#o` mandatory. A
/// signature without one is not a JAdES signature, so producing one would be
/// producing something mislabelled.
#[test]
fn an_empty_certificate_chain_is_refused() {
    let h = JadesHeader {
        certificate: CertificateRef::Chain(vec![]),
        ..header()
    };
    assert_eq!(
        prepare(&h, b"payload").unwrap_err(),
        JadesError::EmptyCertificateChain
    );
}

/// `x5c` carries the chain as a JSON array, signing certificate first.
#[test]
fn x5c_carries_the_chain_in_order() {
    let h = JadesHeader {
        certificate: CertificateRef::Chain(vec!["c3ViamVjdA==".into(), "aXNzdWVy".into()]),
        ..header()
    };
    let prepared = prepare(&h, b"payload").expect("prepares");
    let decoded = decode_header(prepared.assemble(&[0u8; 64]).as_str());
    assert_eq!(
        decoded["x5c"],
        serde_json::json!(["c3ViamVjdA==", "aXNzdWVy"]),
        "x5c is an ordered array, signing certificate first (RFC 7515 4.1.6)"
    );
    assert!(
        decoded.get("x5t#S256").is_none(),
        "one reference form, not both, unless deliberately migrating"
    );
}

/// `x5t#S256` is the base64url SHA-256 of the DER certificate.
///
/// RFC 7515 clause 4.1.8. Checked against an independently computed digest
/// rather than against whatever the constructor happened to produce.
#[test]
fn the_thumbprint_is_the_sha256_of_the_der() {
    use sha2::{Digest, Sha256};
    let CertificateRef::Thumbprint(t) = CertificateRef::thumbprint_of_der(FAKE_DER) else {
        panic!("constructed a thumbprint");
    };
    assert_eq!(t, B64.encode(Sha256::digest(FAKE_DER)));
    assert!(!t.contains('='), "base64url without padding");
}

/// `cty` appears only when set.
#[test]
fn content_type_is_optional_and_emitted_when_present() {
    let plain = prepare(&header(), b"p").expect("prepares");
    assert!(
        decode_header(plain.assemble(&[0u8; 64]).as_str())
            .get("cty")
            .is_none()
    );

    let typed = prepare(&header().with_content_type("json"), b"p").expect("prepares");
    assert_eq!(
        decode_header(typed.assemble(&[0u8; 64]).as_str())["cty"],
        "json"
    );
}

/// The header bytes are stable across calls.
///
/// They are signed, so two serialisations that differ only in key order are two
/// different signing inputs. A verifier recomputing the header from a
/// round-tripped structure would then disagree with the signature for no
/// reason a reader could see.
#[test]
fn header_serialisation_is_stable() {
    let h = header().with_content_type("json");
    let once = h.to_json_bytes().expect("serialises");
    for _ in 0..64 {
        assert_eq!(h.to_json_bytes().expect("serialises"), once);
    }
}

// ─── Table 1 conformance, mechanically ───────────────────────────────────────

/// Check a compact JAdES against every JAdES-B-B rule this module claims to
/// implement, returning the rules it broke.
///
/// The per-rule tests above each pin one clause in isolation. This applies all
/// of them to one artefact at once, so a header that satisfies each rule
/// separately but not together is still caught — and a new test case gets the
/// whole checklist rather than whichever assertions its author remembered.
///
/// It is still *our* reading of the standard. The check that is not ours lives
/// in `.github/oracle/jades/`, which hands an artefact to the European
/// Commission's reference implementation and asks what it is.
fn table_1_b_b_violations(compact: &str) -> Vec<String> {
    let mut bad = Vec::new();
    let parts: Vec<&str> = compact.split('.').collect();
    if parts.len() != 3 {
        return vec![format!(
            "compact form has {} segments, expected 3",
            parts.len()
        )];
    }

    let Ok(header_bytes) = B64.decode(parts[0]) else {
        return vec!["protected header is not base64url".to_owned()];
    };
    let Ok(h) = serde_json::from_slice::<Value>(&header_bytes) else {
        return vec!["protected header is not JSON".to_owned()];
    };
    let Some(obj) = h.as_object() else {
        return vec!["protected header is not a JSON object".to_owned()];
    };

    // Table 1: alg shall be present, cardinality 1.
    if !obj.get("alg").is_some_and(Value::is_string) {
        bad.push("alg absent or not a string (Table 1, clause 5.1.2)".to_owned());
    }

    // Clause 5.1.7: at least one signing-certificate reference.
    let refs = ["x5t#S256", "x5c", "sigX5ts", "x5t#o"];
    if !refs.iter().any(|k| obj.contains_key(*k)) {
        bad.push(format!(
            "no signing-certificate reference; need one of {refs:?} (clause 5.1.7)"
        ));
    }

    // Clause 5.1.11: iat present, integer, no fractional part.
    match obj.get("iat") {
        None => bad.push("iat absent (clause 5.1.11, mandatory since 2025-07-15)".to_owned()),
        Some(v) if !v.is_i64() => bad.push(format!("iat is not an integer: {v}")),
        Some(_) => {}
    }

    // Clause 5.2.8.1: sigD shall not appear when the payload is attached, and
    // everything this module builds is attached.
    if obj.contains_key("sigD") {
        bad.push("sigD present on an attached payload (clause 5.2.8.1)".to_owned());
    }

    // Clause 5.1.9: crit is required *only* alongside sigD. Emitting it without
    // sigD does not break the letter, but it breaks the intent — V1.2.1
    // suppressed the blanket rule so a JAdES without sigD stays processable by
    // a plain JWS library, and a gratuitous crit takes that back.
    if obj.contains_key("crit") && !obj.contains_key("sigD") {
        bad.push("crit present without sigD (clause 5.1.9 NOTE 1)".to_owned());
    }

    if B64.decode(parts[1]).is_err() {
        bad.push("payload segment is not base64url".to_owned());
    }
    if B64.decode(parts[2]).is_err() {
        bad.push("signature segment is not base64url".to_owned());
    }
    bad
}

/// Every shape this module can produce satisfies Table 1 at B-B.
///
/// The payload cases are the ones most likely to break an encoder: empty,
/// binary that is not valid UTF-8, multi-byte characters, and something large
/// enough to cross buffer boundaries.
#[test]
fn every_producible_shape_satisfies_table_1() {
    let payloads: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"{}".to_vec(),
        vec![0x00, 0xff, 0xfe, 0x80, 0x7f],
        "sector: electrique — battery".as_bytes().to_vec(),
        vec![b'x'; 64 * 1024],
    ];
    let headers = vec![
        header(),
        header().with_content_type("json"),
        JadesHeader {
            certificate: CertificateRef::Chain(vec!["Y2VydA==".into(), "aXNzdWVy".into()]),
            ..header()
        },
        JadesHeader {
            alg: "ES256".into(),
            ..header()
        },
    ];

    let mut checked = 0usize;
    for h in &headers {
        for p in &payloads {
            let prepared = prepare(h, p).expect("prepares");
            let compact = prepared.assemble(&[7u8; 64]).into_string();
            let violations = table_1_b_b_violations(&compact);
            assert!(
                violations.is_empty(),
                "alg={} payload={} bytes violated: {violations:?}",
                h.alg,
                p.len()
            );
            checked += 1;
        }
    }
    assert_eq!(
        checked, 20,
        "every header/payload combination was exercised"
    );
}

/// The checker rejects what it is supposed to reject.
///
/// A conformance checker nobody has watched fail is a checker that might be
/// asserting nothing. Each case below changes exactly one thing.
#[test]
fn the_table_1_checker_catches_each_violation() {
    fn with_header(mutate: impl FnOnce(&mut serde_json::Map<String, Value>)) -> String {
        let prepared = prepare(&header(), b"payload").expect("prepares");
        let compact = prepared.assemble(&[0u8; 64]).into_string();
        let parts: Vec<&str> = compact.split('.').collect();
        let mut h: serde_json::Map<String, Value> =
            serde_json::from_slice(&B64.decode(parts[0]).expect("b64")).expect("json");
        mutate(&mut h);
        format!(
            "{}.{}.{}",
            B64.encode(serde_json::to_vec(&Value::Object(h)).expect("json")),
            parts[1],
            parts[2]
        )
    }

    let cases: Vec<(&str, String, &str)> = vec![
        (
            "missing alg",
            with_header(|h| {
                h.remove("alg");
            }),
            "alg absent",
        ),
        (
            "missing certificate reference",
            with_header(|h| {
                h.remove("x5t#S256");
            }),
            "signing-certificate reference",
        ),
        (
            "missing iat",
            with_header(|h| {
                h.remove("iat");
            }),
            "iat absent",
        ),
        (
            "fractional iat",
            with_header(|h| {
                h.insert("iat".into(), serde_json::json!(1_770_000_000.5_f64));
            }),
            "not an integer",
        ),
        (
            "sigD on an attached payload",
            with_header(|h| {
                h.insert("sigD".into(), serde_json::json!({"pars": []}));
            }),
            "sigD present",
        ),
        (
            "crit without sigD",
            with_header(|h| {
                h.insert("crit".into(), serde_json::json!(["iat"]));
            }),
            "crit present without sigD",
        ),
    ];

    for (name, compact, expected) in cases {
        let violations = table_1_b_b_violations(&compact);
        assert!(
            violations.iter().any(|v| v.contains(expected)),
            "{name}: expected a violation mentioning {expected:?}, got {violations:?}"
        );
    }

    // The unmodified artefact passes, so each case above fails for the reason
    // stated rather than because everything fails.
    let clean = prepare(&header(), b"payload")
        .expect("prepares")
        .assemble(&[0u8; 64])
        .into_string();
    assert!(table_1_b_b_violations(&clean).is_empty());
}

/// `now()` claims a signing time in whole seconds, close to now.
#[test]
fn now_claims_a_plausible_whole_second_signing_time() {
    let before = chrono::Utc::now().timestamp();
    let h = JadesHeader::now("EdDSA", CertificateRef::thumbprint_of_der(FAKE_DER));
    let after = chrono::Utc::now().timestamp();
    assert!(
        (before..=after).contains(&h.iat),
        "iat {} is outside [{before}, {after}]",
        h.iat
    );
    let compact = prepare(&h, b"p").expect("prepares").assemble(&[0u8; 64]);
    assert!(table_1_b_b_violations(compact.as_str()).is_empty());
}

/// The module is indifferent to signature length, because the algorithm is the
/// caller's business.
///
/// `ES256` produces 64 bytes, `RS256` 256, Ed25519 64. Baking in an expectation
/// would silently restrict which providers could be used — the opposite of the
/// point of building the format ourselves.
#[test]
fn signature_length_is_not_constrained() {
    for len in [64usize, 96, 128, 256, 384, 512] {
        let compact = prepare(&header(), b"payload")
            .expect("prepares")
            .assemble(&vec![0xabu8; len])
            .into_string();
        assert!(table_1_b_b_violations(&compact).is_empty(), "len {len}");
        let sig = B64
            .decode(compact.split('.').nth(2).expect("sig segment"))
            .expect("b64");
        assert_eq!(sig.len(), len, "the signature round-trips at {len} bytes");
    }
}

/// An empty payload still produces a well-formed signature.
///
/// `base64url("")` is the empty string, so the compact form has an empty middle
/// segment — legal in JWS, and the kind of thing a naive assembler gets wrong.
#[test]
fn an_empty_payload_produces_an_empty_middle_segment() {
    let compact = prepare(&header(), b"")
        .expect("prepares")
        .assemble(&[0u8; 64])
        .into_string();
    let parts: Vec<&str> = compact.split('.').collect();
    assert_eq!(parts.len(), 3);
    assert!(parts[1].is_empty(), "an empty payload encodes to nothing");
    assert!(table_1_b_b_violations(&compact).is_empty());
}

/// No segment carries base64 padding.
///
/// RFC 7515 clause 2 requires base64url **without** padding. A `=` anywhere is
/// a signature other implementations will reject.
#[test]
fn no_segment_carries_base64_padding() {
    // A payload length that is not a multiple of three, so padding would show
    // if the encoder emitted any.
    let compact = prepare(&header().with_content_type("json"), b"1234567890abcdefghij")
        .expect("prepares")
        .assemble(&[9u8; 64])
        .into_string();
    assert!(
        !compact.contains('='),
        "base64url in JWS is unpadded (RFC 7515 clause 2): {compact}"
    );
}

/// The combined form emits both `x5c` and `x5t#S256`.
///
/// Clause 5.1.7 permits either alone; Table 1's "signing a reference of the
/// signing certificate" service admits only the digest forms. A signature with
/// `x5c` alone therefore satisfies 5.1.7 and is still not baseline — the
/// European Commission's DSS reported exactly that as `JSON-NOT-ETSI`, warning
/// that the signing-certificate attribute was absent.
///
/// Found by an outside implementation rather than by reading, which is the
/// whole reason the oracle exists.
#[test]
fn the_combined_form_carries_the_chain_and_the_digest() {
    let der = FAKE_DER.to_vec();
    let cert = CertificateRef::chain_of_der(&[der.clone()]).expect("one certificate");
    let h = JadesHeader {
        certificate: cert,
        ..header()
    };
    let decoded = decode_header(
        prepare(&h, b"payload")
            .expect("prepares")
            .assemble(&[0u8; 64])
            .as_str(),
    );

    assert!(
        decoded.get("x5c").is_some(),
        "the chain travels with the signature"
    );
    let CertificateRef::Thumbprint(expected) = CertificateRef::thumbprint_of_der(&der) else {
        unreachable!()
    };
    assert_eq!(
        decoded["x5t#S256"],
        serde_json::json!(expected),
        "and the digest reference Table 1 requires for baseline"
    );
    assert!(
        table_1_b_b_violations(
            prepare(&h, b"payload")
                .expect("prepares")
                .assemble(&[0u8; 64])
                .as_str()
        )
        .is_empty()
    );
}

/// An empty chain is refused by the combined constructor too.
#[test]
fn the_combined_constructor_refuses_an_empty_chain() {
    assert_eq!(
        CertificateRef::chain_of_der(&[]).unwrap_err(),
        JadesError::EmptyCertificateChain
    );
}
