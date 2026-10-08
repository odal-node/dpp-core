//! Tokens another implementation issued, read by this crate.
//!
//! The other direction of the oracle. `.github/oracle/sdjwt/issue.mjs` issues
//! tokens with an independent SD-JWT library: object properties, nested and
//! recursive ones, array elements, decoys, a Key Binding JWT. This crate has to
//! verify the signature, process the Disclosures and arrive at what that library
//! says the claims are. That is the half the RFC's examples cannot give, because
//! the examples are one issuer's output and this crate's own tokens are the other
//! direction.
//!
//! ```text
//! CHECK_SDJWT_ORACLE_ISSUED=target/sdjwt-oracle/issued.json \
//!   cargo test -p dpp-crypto --lib sd_jwt::oracle_tests::reverse
//! ```
//!
//! Without the variable the first test does nothing, because there is no file to
//! read. The second runs regardless and shows that the check itself fails on a
//! mismatch, so a green run of the first means something.

use serde_json::Value;

use crate::jws::verifier::verify_jws;
use crate::sd_jwt::SdJwt;

/// Every way `file` disagrees with this crate.
///
/// `file` is `{ "publicKey": <base64url raw Ed25519 key>, "cases": [...] }`, and a
/// case is `{ "id", "verdict": "accept" | "refuse", "token", "expected"? }`.
fn disagreements(file: &Value) -> Vec<String> {
    let key = file["publicKey"].as_str().expect("the file names its key");
    let cases = file["cases"].as_array().expect("the file holds cases");
    let mut out = Vec::new();
    for case in cases {
        let id = case["id"].as_str().unwrap_or("?");
        let token = case["token"].as_str().expect("a case has a token");
        let reading = SdJwt::parse(token)
            .map_err(|e| format!("{e:?}"))
            .and_then(|sd_jwt| {
                match verify_jws(sd_jwt.jwt(), key) {
                    Ok(true) => {}
                    other => return Err(format!("signature: {other:?}")),
                }
                let payload = sd_jwt.disclosed_payload().map_err(|e| format!("{e:?}"))?;
                Ok((sd_jwt.has_key_binding(), Value::Object(payload)))
            });
        match (case["verdict"].as_str(), reading) {
            (Some("accept"), Ok((has_kb, payload))) => {
                if payload != case["expected"] {
                    out.push(format!(
                        "{id}: read {payload}, the other implementation says {}",
                        case["expected"]
                    ));
                }
                if case["keyBinding"].as_bool().is_some_and(|kb| kb != has_kb) {
                    out.push(format!("{id}: key binding present is {has_kb}"));
                }
            }
            (Some("accept"), Err(why)) => {
                out.push(format!("{id}: refused a token the other issued: {why}"));
            }
            (Some("refuse"), Ok(_)) => {
                out.push(format!("{id}: accepted a token this crate does not read"));
            }
            (Some("refuse"), Err(_)) => {}
            (other, _) => out.push(format!("{id}: unknown verdict {other:?}")),
        }
    }
    out
}

#[test]
fn what_an_independent_implementation_issued_reads_back_here() {
    let Some(path) = std::env::var_os("CHECK_SDJWT_ORACLE_ISSUED") else {
        return;
    };
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.to_string_lossy()));
    let file: Value = serde_json::from_str(&text).expect("the file is JSON");

    let count = file["cases"].as_array().map_or(0, Vec::len);
    assert!(count >= 100, "only {count} tokens to read back");
    let found = disagreements(&file);
    assert!(
        found.is_empty(),
        "{} of {count} tokens issued by an independent implementation do not read \
         back the way it says:\n{}",
        found.len(),
        found.join("\n")
    );
    eprintln!("read back {count} tokens issued by an independent implementation");
}

#[test]
fn the_readback_check_fails_on_a_mismatch() {
    use serde_json::json;

    use super::plan::{issue_to_plan, paths};
    use super::signing::Issuer;

    let issuer = Issuer::new();
    let claims = json!({"a": 1, "b": ["x", "y"]});
    let issued = issue_to_plan(&claims, &paths(&[&["a"], &["b", "#1"]]));
    let encoded: Vec<String> = issued
        .disclosures
        .iter()
        .map(|d| d.encoded().to_owned())
        .collect();
    let token = issuer.token(&issued.payload, &encoded);

    let file = |verdict: &str, expected: Value| {
        json!({
            "publicKey": issuer.public_key_b64,
            "cases": [{"id": "case", "verdict": verdict, "token": token, "expected": expected}]
        })
    };

    assert!(disagreements(&file("accept", claims.clone())).is_empty());
    assert_eq!(
        disagreements(&file("accept", json!({"a": 2, "b": ["x", "y"]}))).len(),
        1,
        "a wrong expectation must be reported"
    );
    assert_eq!(
        disagreements(&file("refuse", Value::Null)).len(),
        1,
        "a token claimed unreadable that reads fine must be reported"
    );

    let wrong_key = json!({
        "publicKey": Issuer::new().public_key_b64,
        "cases": [{"id": "case", "verdict": "accept", "token": token, "expected": claims}]
    });
    assert_eq!(
        disagreements(&wrong_key).len(),
        1,
        "a token under another key must be reported"
    );
}
