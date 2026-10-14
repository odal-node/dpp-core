//! Tokens another implementation issued, read by this crate.
//!
//! The other direction of the oracle. `.github/oracle/sdjwt/issue.mjs` issues
//! tokens with an independent SD-JWT library: object properties, nested and
//! recursive ones, array elements, decoys, a Key Binding JWT. This crate has to
//! verify the signature, process the Disclosures and arrive at what that library
//! says the claims are. A token bound to a Holder's key comes with the challenge
//! the library verified it against, and this crate validates its Key Binding JWT
//! against the same one. That is the half the RFC's examples cannot give, because
//! the examples are one issuer's output and this crate's own tokens are the other
//! direction.
//!
//! A token the library presented from an issued one also comes with the issued token
//! and the claim paths it was asked to present. Choosing the Disclosures for those
//! paths is the library's job in the other direction, and this crate's
//! [`SdJwt::digests_for_paths`] has to choose the same ones.
//!
//! ```text
//! CHECK_SDJWT_ORACLE_ISSUED=target/sdjwt-oracle/issued.json \
//!   cargo test -p dpp-crypto --lib sd_jwt::oracle_tests::reverse
//! ```
//!
//! Without the variable the first test does nothing, because there is no file to
//! read. The second runs regardless and shows that the check itself fails on a
//! mismatch, so a green run of the first means something.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

use crate::jws::verifier::verify_jws;
use crate::sd_jwt::{ClaimPath, Disclosure, KeyBindingPolicy, SdJwt};

/// Every way `file` disagrees with this crate.
///
/// `file` is `{ "publicKey": <base64url raw Ed25519 key>, "cases": [...] }`, and a
/// case is `{ "id", "verdict": "accept" | "refuse", "token", "expected"?,
/// "challenge"?, "selection"? }`. A case with a `challenge` (`nonce`, `aud`, `now`,
/// `window`) is read by a Verifier that requires Key Binding. A case with a
/// `selection` (`of`: the issued token, `paths`: claim paths) was presented from `of`
/// by the other library, and this crate has to choose the same Disclosures.
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
                let payload = match challenge(&case["challenge"]) {
                    Some(policy) => sd_jwt.disclosed_payload_with_key_binding(&policy),
                    None => sd_jwt.disclosed_payload(),
                }
                .map_err(|e| format!("{e:?}"))?;
                Ok((sd_jwt.has_key_binding(), Value::Object(payload)))
            });
        match (case["verdict"].as_str(), reading) {
            (Some("accept"), Ok((has_kb, payload))) => {
                if let Some(selection) = case.get("selection")
                    && let Some(why) = selection_disagreement(selection, token)
                {
                    out.push(format!("{id}: {why}"));
                }
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

/// How this crate's choice of Disclosures for a case's claim paths differs from the
/// Disclosures the other library presented, if it does.
fn selection_disagreement(selection: &Value, presented: &str) -> Option<String> {
    let issued = SdJwt::parse(selection["of"].as_str()?).ok()?;
    let theirs: BTreeSet<String> = SdJwt::parse(presented)
        .ok()?
        .disclosures()
        .iter()
        .map(Disclosure::digest)
        .collect();
    let paths: Result<Vec<ClaimPath>, _> = selection["paths"]
        .as_array()?
        .iter()
        .map(ClaimPath::from_json)
        .collect();
    let Ok(paths) = paths else {
        return Some(format!(
            "{} holds a claim path that is not one",
            selection["paths"]
        ));
    };
    match issued.digests_for_paths(&paths) {
        Ok(ours) => {
            let ours: BTreeSet<String> = ours.into_iter().collect();
            (ours != theirs).then(|| {
                format!(
                    "for {} this crate chooses {} Disclosure(s), the other library presented {}",
                    selection["paths"],
                    ours.len(),
                    theirs.len()
                )
            })
        }
        Err(e) => Some(format!(
            "the other library presented {} Disclosure(s) for {}, and this crate says: {e}",
            theirs.len(),
            selection["paths"]
        )),
    }
}

/// The policy a case's `challenge` describes, when it has one.
fn challenge(value: &Value) -> Option<KeyBindingPolicy> {
    let window = u32::try_from(value["window"].as_u64()?).ok()?;
    Some(KeyBindingPolicy {
        audience: value["aud"].as_str()?.to_owned(),
        nonce: value["nonce"].as_str()?.to_owned(),
        now: value["now"].as_i64()?,
        max_age: window,
        leeway: window,
    })
}

#[test]
fn what_an_independent_implementation_issued_reads_back_here() {
    let Some(given) = std::env::var_os("CHECK_SDJWT_ORACLE_ISSUED") else {
        return;
    };
    // Tests run in the crate directory, so a relative path is read from the
    // workspace root, where `emit` writes and where the command is run from.
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(given);
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let file: Value = serde_json::from_str(&text).expect("the file is JSON");

    let count = file["cases"].as_array().map_or(0, Vec::len);
    assert!(count >= 100, "only {count} tokens to read back");
    // A file with no selections would pass the choice check by never making it.
    let selections = file["cases"].as_array().map_or(0, |cases| {
        cases
            .iter()
            .filter(|c| c.get("selection").is_some())
            .count()
    });
    assert!(
        selections >= 20,
        "only {selections} presentations to choose the Disclosures of"
    );
    let found = disagreements(&file);
    assert!(
        found.is_empty(),
        "{} of {count} tokens issued by an independent implementation do not read \
         back the way it says:\n{}",
        found.len(),
        found.join("\n")
    );
    eprintln!(
        "read back {count} tokens issued by an independent implementation, \
         and chose the same Disclosures for {selections} presentations of them"
    );
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

    // A presentation is held to the claim paths it was made for. The paths it was
    // made for agree, other paths are reported, and so is a path not in the token.
    let full = SdJwt::parse(&token).expect("parses");
    let only_a: Vec<String> = full
        .disclosures()
        .iter()
        .filter(|d| d.claim_name() == Some("a"))
        .map(Disclosure::digest)
        .collect();
    let presented_a = full
        .present(&only_a.iter().map(String::as_str).collect::<Vec<_>>())
        .expect("presents")
        .serialise();
    let selection_file = |paths: Value| {
        json!({
            "publicKey": issuer.public_key_b64,
            "cases": [{
                "id": "selected", "verdict": "accept", "token": presented_a,
                "expected": {"a": 1, "b": ["x"]},
                "selection": { "of": token, "paths": paths },
            }]
        })
    };
    assert!(disagreements(&selection_file(json!([["a"]]))).is_empty());
    assert_eq!(
        disagreements(&selection_file(json!([["b", 1]]))).len(),
        1,
        "a presentation made for other paths must be reported"
    );
    assert_eq!(
        disagreements(&selection_file(json!([["nope"]]))).len(),
        1,
        "a path this crate cannot find must be reported"
    );

    // A bound token is read with its Key Binding JWT checked: under the challenge
    // it answers it reads back, and under another nonce it is reported.
    let holder = super::signing::holder("sdjwt-oracle-reverse-holder");
    let mut bound_claims = claims.clone();
    bound_claims["cnf"] = crate::sd_jwt::holder_key_confirmation(&holder.verifying_key.to_bytes());
    let plan = issue_to_plan(&bound_claims, &paths(&[&["a"]]));
    let issued = SdJwt::new(issuer.sign(&plan.payload), plan.disclosures);
    let binding = crate::sd_jwt::KeyBinding {
        audience: "https://verifier.example".to_owned(),
        nonce: "n".to_owned(),
        issued_at: 1_800_000_000,
    };
    let reveal: Vec<String> = issued.disclosures().iter().map(|d| d.digest()).collect();
    let reveal: Vec<&str> = reveal.iter().map(String::as_str).collect();
    let bound = issued
        .present_with_key_binding(&reveal, &binding, &holder)
        .expect("binds")
        .serialise();
    let bound_file = |nonce: &str| {
        json!({
            "publicKey": issuer.public_key_b64,
            "cases": [{
                "id": "bound", "verdict": "accept", "token": bound, "expected": bound_claims,
                "challenge": { "nonce": nonce, "aud": "https://verifier.example", "now": 1_800_000_000, "window": 0 },
            }]
        })
    };
    assert!(disagreements(&bound_file("n")).is_empty());
    assert_eq!(
        disagreements(&bound_file("another")).len(),
        1,
        "a Key Binding JWT that answers another challenge must be reported"
    );
}
