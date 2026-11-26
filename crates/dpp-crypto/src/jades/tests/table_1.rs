//! Table 1 of the standard, checked against every producible shape.

use base64::Engine;
use serde_json::Value;

use super::super::*;
use super::common::*;

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
        "product_group: electrique — battery".as_bytes().to_vec(),
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
