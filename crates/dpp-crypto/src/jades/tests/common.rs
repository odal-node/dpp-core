//! Helpers the JAdES tests share.

use base64::Engine;
use serde_json::Value;

use super::super::*;

pub(super) const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// A stand-in for a DER certificate. The bytes are never parsed by this module
/// — `x5t#S256` digests them and `x5c` carries them — so a fixed blob is enough
/// to exercise the format without pulling in a certificate generator.
pub(super) const FAKE_DER: &[u8] = b"\x30\x82\x01\x0a-not-a-real-certificate-";

pub(super) fn header() -> JadesHeader {
    JadesHeader {
        alg: "EdDSA".into(),
        iat: 1_770_000_000,
        certificate: CertificateRef::thumbprint_of_der(FAKE_DER),
        content_type: None,
    }
}

pub(super) fn decode_header(compact: &str) -> Value {
    let seg = compact.split('.').next().expect("has a header segment");
    serde_json::from_slice(&B64.decode(seg).expect("header is base64url")).expect("header is JSON")
}

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
pub(super) fn table_1_b_b_violations(compact: &str) -> Vec<String> {
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
