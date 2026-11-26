//! Build the COSE corpus, check this crate against its own plan, and write it out.
//!
//! ```text
//! EMIT_COSE_CORPUS=1 cargo test -p dpp-vc --lib jose::tests::oracle::cose::emit
//! ```
//!
//! Writes `target/cose-oracle/corpus.json`, which `.github/oracle/cose` reads. Without the
//! variable the test still runs and still checks that the corpus is complete and that this
//! crate verifies every message in it, so an ordinary `just check` gets that coverage without
//! producing files.
//!
//! Every message is signed with the suite's own key for its algorithm, so the file is the same
//! on every run: ECDSA here is deterministic (RFC 6979).

use std::path::Path;

use base64::Engine as _;
use serde_json::{Value, json};

use super::super::high_s;
use super::super::keys::{self, ALGORITHMS};
use super::message::{self, name_of, parts};
use crate::jose::VerifyOptions;
use crate::jose::{
    sign_credential_cose, sign_presentation_cose, verify_credential_cose, verify_presentation_cose,
};
use crate::test_support::NOT_REQUIRED;
use crate::test_support::{
    cose_envelope, credential, json as suite_json, key_id, presentation, public_key, signing_key,
};
use dpp_crypto::jws::PublicKey;

fn standard(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// One message and what the judge needs to know about it.
fn case(
    id: String,
    group: &str,
    kind: &str,
    alg: &str,
    kid: &str,
    message: &[u8],
    document: &Value,
) -> Value {
    json!({
        "id": id,
        "group": group,
        "kind": kind,
        "alg": alg,
        "kid": kid,
        "message": standard(message),
        "document": document,
    })
}

/// The suite's issuance inputs, rows 27 and 28, signed as `suite_cose_tests` signs them.
fn suite_cases() -> Vec<Value> {
    let credential = suite_json("credential-minimal.json");
    let message = sign_credential_cose(
        &credential,
        &signing_key("did-ed25519"),
        &key_id("did-ed25519"),
        &[],
    )
    .expect("the suite input is issued");
    let presentation = suite_json("presentation-single.json");
    let presented =
        sign_presentation_cose(&presentation, &signing_key("p384"), &key_id("p384"), &[])
            .expect("the suite input is issued");
    vec![
        case(
            "suite/row-27".into(),
            "suite",
            "vc",
            "EdDSA",
            &key_id("did-ed25519"),
            &message,
            &credential,
        ),
        case(
            "suite/row-28".into(),
            "suite",
            "vp",
            "ES384",
            &key_id("p384"),
            &presented,
            &presentation,
        ),
    ]
}

/// Credentials of four shapes: the base one, one whose issuer is an object, one with
/// non-ASCII text and nesting, and one with the numbers JCS writes by its own rule.
fn credentials() -> Vec<(&'static str, Value)> {
    let mut object = credential();
    object["issuer"] = json!({"id": "https://issuer.example", "name": "Example"});
    object["validUntil"] = json!("2099-01-01T00:00:00Z");
    let mut text = credential();
    text["credentialSubject"] = json!([
        {"id": "did:example:one", "name": "日本語 ✓ 😀", "nested": {"a": [1, {"b": null}], "c": "\u{7f}\"\\<>&"}},
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
        ("issuer-object", object),
        ("text", text),
        ("numbers", numbers),
    ]
}

/// Every credential under every algorithm, and for every algorithm two presentations: one
/// with no credential, and one that envelopes a credential signed under each of the four
/// algorithms.
fn generated_cases() -> Vec<Value> {
    let mut cases = Vec::new();
    for (alg, _) in ALGORITHMS {
        let (key, kid) = (keys::signing(alg), keys::kid(alg));
        for (shape, document) in credentials() {
            let message = sign_credential_cose(&document, &key, &kid, &[]).expect("signs");
            cases.push(case(
                format!("credential/{alg}/{shape}"),
                "credential",
                "vc",
                alg,
                &kid,
                &message,
                &document,
            ));
        }
        let empty = presentation(Vec::new());
        let message = sign_presentation_cose(&empty, &key, &kid, &[]).expect("signs");
        cases.push(case(
            format!("presentation/{alg}/empty"),
            "presentation",
            "vp",
            alg,
            &kid,
            &message,
            &empty,
        ));
        let mut envelopes = Vec::new();
        let mut listed = Vec::new();
        for (index, (issuer, _)) in ALGORITHMS.iter().enumerate() {
            let inner = sign_credential_cose(
                &credential(),
                &keys::signing(issuer),
                &keys::kid(issuer),
                &[],
            )
            .expect("signs");
            envelopes.push(cose_envelope(&inner));
            listed
                .push(json!({"index": index, "kid": keys::kid(issuer), "document": credential()}));
        }
        let full = presentation(envelopes);
        let message = sign_presentation_cose(&full, &key, &kid, &[]).expect("signs");
        let mut with_envelopes = case(
            format!("presentation/{alg}/four-credentials"),
            "presentation",
            "vp",
            alg,
            &kid,
            &message,
            &full,
        );
        with_envelopes["envelopes"] = json!(listed);
        cases.push(with_envelopes);
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

/// The key a `kid` of the corpus names: the suite's five keys, by the id of each.
fn key_for(kid: &str) -> PublicKey {
    ["ed25519", "p256", "p384", "p521", "did-ed25519"]
        .into_iter()
        .find(|file| key_id(file) == kid)
        .map(public_key)
        .unwrap_or_else(|| panic!("no suite key has the id {kid}"))
}

#[test]
fn the_corpus_is_complete_and_this_crate_verifies_every_message_in_it() {
    let cases = build();

    for case in &cases {
        let (id, alg) = (text(case, "id"), text(case, "alg"));
        let message = base64::engine::general_purpose::STANDARD
            .decode(text(case, "message"))
            .expect("padded base64");
        assert_eq!(name_of(parts(&message).alg), alg, "{id}");
        let key = key_for(text(case, "kid"));
        match text(case, "kind") {
            "vc" => assert_eq!(
                verify_credential_cose(&message, &key, VerifyOptions::new()).as_ref(),
                Ok(&case["document"]),
                "{id}"
            ),
            _ => {
                let verified = verify_presentation_cose(&message, &key, VerifyOptions::new())
                    .unwrap_or_else(|e| panic!("{id}: {e}"));
                assert_eq!(verified.payload(), &case["document"], "{id}");
                // Every enveloped credential in a presentation of this crate's own verifies
                // under the key the corpus lists for it.
                if let Some(listed) = case["envelopes"].as_array() {
                    let read = verified
                        .verify_credentials(VerifyOptions::new(), &NOT_REQUIRED, |e| {
                            Some(key_for(text(&listed[e.index], "kid")))
                        })
                        .unwrap_or_else(|e| panic!("{id}: {e}"));
                    assert_eq!(read.len(), listed.len(), "{id}");
                    assert_eq!(verified.envelopes().len(), listed.len(), "{id}");
                }
            }
        }
        // This crate writes the lower `S`; the library is asked to read it.
        if high_s::is_ecdsa(alg) {
            assert!(
                !high_s::is_high_s(alg, &message::signature(&message)),
                "{id}: this crate signs low-S"
            );
        }
    }

    // A count, because a loop over an empty corpus passes.
    assert_eq!(count(&cases, "suite"), 2, "the suite's COSE issuance rows");
    assert_eq!(
        count(&cases, "credential"),
        16,
        "4 shapes under 4 algorithms"
    );
    assert_eq!(count(&cases, "presentation"), 8, "2 under each algorithm");
    let envelopes: usize = cases
        .iter()
        .map(|c| c["envelopes"].as_array().map_or(0, Vec::len))
        .sum();
    assert_eq!(
        envelopes, 16,
        "4 enveloped credentials in each of 4 presentations"
    );

    emit(&cases);
}

/// Writes the corpus when `EMIT_COSE_CORPUS` is set.
fn emit(cases: &[Value]) {
    if std::env::var_os("EMIT_COSE_CORPUS").is_none() {
        return;
    }
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/cose-oracle");
    std::fs::create_dir_all(&dir).expect("create the oracle output directory");
    let body = json!({ "cases": cases });
    std::fs::write(
        dir.join("corpus.json"),
        serde_json::to_string(&body).expect("serialises"),
    )
    .expect("write the corpus");
    eprintln!(
        "wrote {} cases ({} suite, {} credentials, {} presentations) to {}",
        cases.len(),
        count(cases, "suite"),
        count(cases, "credential"),
        count(cases, "presentation"),
        dir.display()
    );
}
