//! Build the corpus, check this crate against its own plan, and write it out.
//!
//! ```text
//! EMIT_JOSE_CORPUS=1 cargo test -p dpp-vc --lib jose::tests::oracle::emit
//! ```
//!
//! Writes `target/jose-oracle/corpus.json`, which `.github/oracle/jose/judge.mjs`
//! reads. Without the variable the test still runs and still checks that the
//! corpus is complete and that this crate verifies every token in it, so an
//! ordinary `just check` gets that coverage without producing files.
//!
//! Every token is signed with the suite's own key for its algorithm, so the file is
//! the same on every run: ECDSA here is deterministic (RFC 6979).

use std::path::Path;

use serde_json::{Value, json};

use super::keys::{self, ALGORITHMS};
use super::{high_s, token};
use crate::jose::VerifyOptions;
use crate::jose::{sign_credential, sign_presentation, verify_credential, verify_presentation};
use crate::test_support::NOT_REQUIRED;
use crate::test_support::{credential, envelope, json as suite_json, presentation};

/// One token and what the judge needs to know about it.
fn case(id: String, group: &str, kind: &str, alg: &str, token: String, document: &Value) -> Value {
    json!({
        "id": id,
        "group": group,
        "kind": kind,
        "alg": alg,
        "kid": keys::kid(alg),
        "token": token,
        "document": document,
    })
}

fn with_note(mut case: Value, key: &str, note: &str) -> Value {
    case[key] = json!(note);
    case
}

/// The suite's issuance inputs, rows 1 to 5, signed as `suite_issue_tests` signs
/// them. Row 5 is signed with its two terms declared, since it is refused without.
fn suite_cases() -> Vec<Value> {
    let signed = |alg: &str, input: &str, kind: &str, terms: &[&str]| {
        let document = suite_json(input);
        let (key, kid) = (keys::signing(alg), keys::kid(alg));
        let token = if kind == "vc" {
            sign_credential(&document, &key, &kid, terms)
        } else {
            sign_presentation(&document, &key, &kid, terms)
        }
        .expect("the suite input is issued");
        (token, document)
    };
    let mut cases = Vec::new();
    let (t, d) = signed("ES256", "credential-minimal.json", "vc", &[]);
    cases.push(case("suite/row-1".into(), "suite", "vc", "ES256", t, &d));
    let (t, d) = signed("ES384", "credential-full.json", "vc", &[]);
    cases.push(case("suite/row-2".into(), "suite", "vc", "ES384", t, &d));
    let (t, d) = signed("ES256", "presentation-single.json", "vp", &[]);
    cases.push(with_note(
        case("suite/row-3".into(), "suite", "vp", "ES256", t, &d),
        "innerMalformedBySuite",
        "the suite's envelope is not a compact JWS: its header segment begins yJraWQ, where a JSON object's encoding begins eyJraWQ",
    ));
    let (t, d) = signed("ES512", "presentation-multiple.json", "vp", &[]);
    cases.push(case("suite/row-4".into(), "suite", "vp", "ES512", t, &d));
    let (t, d) = signed(
        "ES512",
        "credential-unknown-extensions.json",
        "vc",
        &["badExtension", "anotherBadOne"],
    );
    cases.push(with_note(
        case("suite/row-5".into(), "suite", "vc", "ES512", t, &d),
        "note",
        "refused at issuance until both terms are declared",
    ));
    cases
}

/// Credentials of four shapes: the base one, one with the JWT claims, one with
/// non-ASCII text and nesting, and one with the numbers JCS writes by its own rule.
fn credentials() -> Vec<(&'static str, Value)> {
    let mut claims = credential();
    claims["iss"] = json!("https://issuer.example");
    claims["nbf"] = json!(1_700_000_000);
    claims["exp"] = json!(4_000_000_000_i64);
    claims["iat"] = json!(1_700_000_000);
    let mut text = credential();
    text["issuer"] = json!({"id": "https://issuer.example", "name": "Zażółć gęślą jaźń"});
    text["credentialSubject"] = json!([
        {"id": "did:example:one", "name": "日本語 ✓ 😀", "nested": {"a": [1, {"b": null}], "c": "\u{7f}\"\\"}},
        {"id": "did:example:two", "name": "Example"},
    ]);
    let mut numbers = credential();
    numbers["credentialSubject"] = json!({
        "id": "did:example:subject",
        "small": 0.000001,
        "large": 1e21,
        "negative": -0.5,
        "integer": 9_007_199_254_740_991_i64,
        "zero": 0,
    });
    vec![
        ("base", credential()),
        ("jwt-claims", claims),
        ("text", text),
        ("numbers", numbers),
    ]
}

/// Every credential under every algorithm, and for every algorithm two
/// presentations: one with no credential, and one that envelopes a credential
/// signed under each of the four algorithms.
fn generated_cases() -> Vec<Value> {
    let mut cases = Vec::new();
    for (alg, _) in ALGORITHMS {
        let (key, kid) = (keys::signing(alg), keys::kid(alg));
        for (shape, document) in credentials() {
            let token = sign_credential(&document, &key, &kid, &[]).expect("signs");
            cases.push(case(
                format!("credential/{alg}/{shape}"),
                "credential",
                "vc",
                alg,
                token,
                &document,
            ));
        }
        let empty = presentation(Vec::new());
        let token = sign_presentation(&empty, &key, &kid, &[]).expect("signs");
        cases.push(case(
            format!("presentation/{alg}/empty"),
            "presentation",
            "vp",
            alg,
            token,
            &empty,
        ));
        let envelopes: Vec<Value> = ALGORITHMS
            .iter()
            .map(|(issuer, _)| {
                let token = sign_credential(
                    &credential(),
                    &keys::signing(issuer),
                    &keys::kid(issuer),
                    &[],
                )
                .expect("signs");
                envelope(&token)
            })
            .collect();
        let full = presentation(envelopes);
        let token = sign_presentation(&full, &key, &kid, &[]).expect("signs");
        cases.push(case(
            format!("presentation/{alg}/four-credentials"),
            "presentation",
            "vp",
            alg,
            token,
            &full,
        ));
    }
    cases
}

fn build() -> Vec<Value> {
    let mut cases = suite_cases();
    cases.extend(generated_cases());
    cases
}

fn count(cases: &[Value], group: &str) -> usize {
    cases.iter().filter(|c| c["group"] == json!(group)).count()
}

fn text<'a>(case: &'a Value, member: &str) -> &'a str {
    case[member].as_str().expect("a string member")
}

/// This crate verifies, with the `kid` the corpus lists, the key that signed.
fn key_for(kid: &str) -> dpp_crypto::jws::PublicKey {
    let published = keys::published();
    dpp_crypto::jws::PublicKey::from_jwk(&published[kid]).expect("a published key")
}

#[test]
fn the_corpus_is_complete_and_this_crate_verifies_every_token_in_it() {
    let cases = build();

    for case in &cases {
        let (id, alg, token) = (text(case, "id"), text(case, "alg"), text(case, "token"));
        let header = token::header(token);
        assert_eq!(header["alg"], json!(alg), "{id}");
        assert_eq!(header["kid"], case["kid"], "{id}");
        let key = keys::public(alg);
        match text(case, "kind") {
            "vc" => assert_eq!(
                verify_credential(token, &key, VerifyOptions::new()).as_ref(),
                Ok(&case["document"]),
                "{id}"
            ),
            _ => {
                let verified = verify_presentation(token, &key, VerifyOptions::new()).expect(id);
                assert_eq!(verified.payload(), &case["document"], "{id}");
                // Every enveloped credential in a presentation of this crate's own
                // verifies under the key its `kid` names.
                if case["group"] == json!("presentation") {
                    let credentials = verified
                        .verify_credentials(VerifyOptions::new(), &NOT_REQUIRED, |e| {
                            Some(key_for(e.kid.as_deref()?))
                        })
                        .expect(id);
                    assert_eq!(credentials.len(), verified.envelopes().len(), "{id}");
                }
            }
        }
        // This crate writes the lower `S`; the library is asked to read it. The
        // enveloped credentials of a presentation of this crate's own are its too.
        let inner = token::envelope_tokens(&case["document"]);
        let mut signed = vec![token];
        if case["group"] == json!("presentation") {
            signed.extend(inner.iter().map(|(_, t)| t.as_str()));
        }
        for token in signed {
            let alg = token::header(token)["alg"]
                .as_str()
                .expect("an alg")
                .to_owned();
            if high_s::is_ecdsa(&alg) {
                assert!(
                    !high_s::is_high_s(&alg, &token::signature(token)),
                    "{id}: this crate signs low-S"
                );
            }
        }
    }

    // A count, because a loop over an empty corpus passes.
    assert_eq!(count(&cases, "suite"), 5, "the suite's issuance rows");
    assert_eq!(
        count(&cases, "credential"),
        16,
        "4 shapes under 4 algorithms"
    );
    assert_eq!(count(&cases, "presentation"), 8, "2 under each algorithm");

    if std::env::var_os("EMIT_JOSE_CORPUS").is_none() {
        return;
    }
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/jose-oracle");
    std::fs::create_dir_all(&dir).expect("create the oracle output directory");
    let body = json!({ "keys": keys::published(), "cases": cases });
    std::fs::write(
        dir.join("corpus.json"),
        serde_json::to_string(&body).expect("serialises"),
    )
    .expect("write the corpus");
    eprintln!(
        "wrote {} cases ({} suite, {} credentials, {} presentations) to {}",
        cases.len(),
        count(&cases, "suite"),
        count(&cases, "credential"),
        count(&cases, "presentation"),
        dir.display()
    );
}
