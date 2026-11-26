//! Table 1 of the standard, checked against every producible shape.

use base64::Engine;
use serde_json::Value;

use super::super::*;
use super::common::*;

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
