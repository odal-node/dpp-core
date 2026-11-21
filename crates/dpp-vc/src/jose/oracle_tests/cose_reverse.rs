//! Messages an independent COSE library signed, verified by this crate.
//!
//! The other direction of the COSE oracle. `.github/oracle/cose` has `go-cose` sign
//! credentials and presentations under EdDSA, ES256, ES384 and ES512 and the suite's own
//! keys. This crate verifies each message with [`verify_credential_cose`] and
//! [`verify_presentation_cose`], and the credentials a presentation envelopes with
//! [`VerifiedPresentation::verify_credentials`](crate::jose::VerifiedPresentation::verify_credentials),
//! and has to arrive at the document the library signed. A case may carry the `now` and the
//! audience a verifier gives, for the message whose CWT claims need them.
//!
//! This crate signs the lower `S` of an ECDSA signature and verifies both. The library does
//! not normalise, so its output holds high-S signatures, and this check holds the file to it:
//! under each ECDSA algorithm, credentials and presentations must each carry at least one
//! high-S signature and at least one low-S one, counted here from the signatures and not from
//! the file's say-so, and the low-S twin of each high-S signature must verify, which shows the
//! curve order used to tell them apart is the right one. The forms are counted per kind, so
//! high-S credentials cannot stand in for high-S presentations. Each algorithm also needs a
//! credential and a presentation that verify, read from each message's own protected header.
//!
//! ```text
//! CHECK_COSE_ORACLE_ISSUED=target/cose-oracle/issued.json \
//!   cargo test -p dpp-vc --lib jose::oracle_tests::cose_reverse
//! ```
//!
//! Without the variable the first test does nothing, because there is no file to read. The
//! second runs regardless: it builds a file of the same shape from this crate's own messages
//! and their high-S twins, and shows that the check accepts it and fails on each way a file can
//! be wrong, so a green run of the first means something.

use std::collections::{BTreeMap, BTreeSet};

use base64::Engine as _;
use serde_json::{Value, json};

use super::cose_message::{name_of, parts, signature, with_signature};
use super::high_s;
use super::keys::{self, ALGORITHMS};
use crate::jose::{VerifyOptions, verify_credential_cose, verify_presentation_cose};
use crate::test_support::NOT_REQUIRED;
use dpp_crypto::cose::verify_sign1;
use dpp_crypto::jws::PublicKey;

/// How many ECDSA signatures of each form a file holds: (kind, algorithm, high-S), where the
/// kind is `credential` or `presentation` and an enveloped credential is a credential.
type Forms = BTreeMap<(&'static str, String, bool), usize>;

fn decode(text: &str) -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode(text)
        .expect("padded standard base64")
}

/// The options a case asks a verifier for.
fn options(case: &Value) -> VerifyOptions<'_> {
    let mut options = VerifyOptions::new();
    if let Some(now) = case["now"].as_i64() {
        options = options.at(now);
    }
    if let Some(audience) = case["audience"].as_str() {
        options = options.for_audience(audience);
    }
    options
}

/// Every way `file` disagrees with this crate, and how many signatures of each form it holds.
///
/// `file` is `{ "keys": { <kid>: <public JWK> }, "cases": [...] }`. A case is `{ "id", "kind":
/// "vc" | "vp", "alg", "kid", "message", "document", "now"?, "audience"?, "envelopes"? }`,
/// where `message` is padded standard base64, `document` is what the library signed, and
/// `envelopes` lists the credentials of a presentation as `{ "index", "kid", "message",
/// "document" }`.
fn read_back(file: &Value) -> (Vec<String>, Forms) {
    let keys = file["keys"].as_object().expect("the file lists its keys");
    let key_of = |kid: &str| keys.get(kid).and_then(PublicKey::from_jwk);
    let cases = file["cases"].as_array().expect("the file holds cases");
    let mut out = Vec::new();
    let mut signatures = Forms::new();
    // The (kind, algorithm) pairs that verified. The algorithm is the message's own, from its
    // protected header, and not the case's `alg` label.
    let mut covered: BTreeSet<(&str, String)> = BTreeSet::new();

    for case in cases {
        let id = case["id"].as_str().unwrap_or("?");
        let message = decode(case["message"].as_str().expect("a case has a message"));
        let kid = case["kid"].as_str().expect("a case has a kid");
        let Some(key) = key_of(kid) else {
            out.push(format!("{id}: no key is published for the kid {kid}"));
            continue;
        };
        let alg = name_of(parts(&message).alg);
        if alg != case["alg"].as_str().unwrap_or("?") {
            out.push(format!(
                "{id}: the message is {alg} and the case says {}",
                case["alg"]
            ));
        }
        let kind = if case["kind"] == json!("vp") {
            "presentation"
        } else {
            "credential"
        };
        tally(id, kind, &message, &key, &mut signatures, &mut out);

        match case["kind"].as_str() {
            Some("vc") => match verify_credential_cose(&message, &key, options(case)) {
                Ok(document) => {
                    covered.insert(("vc", alg.to_owned()));
                    if document != case["document"] {
                        out.push(format!(
                            "{id}: read {document}, the library signed {}",
                            case["document"]
                        ));
                    }
                }
                Err(e) => out.push(format!("{id}: {e:?}")),
            },
            Some("vp") => {
                let verified = match verify_presentation_cose(&message, &key, options(case)) {
                    Ok(verified) => verified,
                    Err(e) => {
                        out.push(format!("{id}: {e:?}"));
                        continue;
                    }
                };
                covered.insert(("vp", alg.to_owned()));
                if verified.payload() != &case["document"] {
                    out.push(format!(
                        "{id}: read {}, the library signed {}",
                        verified.payload(),
                        case["document"]
                    ));
                }
                let listed = case["envelopes"].as_array().cloned().unwrap_or_default();
                if verified.envelopes().len() != listed.len() {
                    out.push(format!(
                        "{id}: the presentation holds {} envelopes and the case lists {}",
                        verified.envelopes().len(),
                        listed.len()
                    ));
                }
                for envelope in &listed {
                    let inner = decode(envelope["message"].as_str().expect("an envelope message"));
                    let index = usize::try_from(envelope["index"].as_u64().unwrap_or(u64::MAX))
                        .unwrap_or(usize::MAX);
                    let in_payload = verified.payload()["verifiableCredential"][index]["id"]
                        .as_str()
                        .unwrap_or_default();
                    let expected = format!(
                        "data:application/vc+cose;base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(&inner)
                    );
                    if in_payload != expected {
                        out.push(format!("{id}: envelope {index} is not the one listed"));
                    }
                    let inner_kid = envelope["kid"].as_str().expect("an envelope kid");
                    match key_of(inner_kid) {
                        Some(inner_key) => {
                            tally(
                                id,
                                "credential",
                                &inner,
                                &inner_key,
                                &mut signatures,
                                &mut out,
                            );
                        }
                        None => out.push(format!("{id}: no key for the kid {inner_kid}")),
                    }
                }
                match verified.verify_credentials(options(case), &NOT_REQUIRED, |e| {
                    key_of(listed.get(e.index)?["kid"].as_str()?)
                }) {
                    Ok(read) => {
                        let signed: Vec<&Value> = listed.iter().map(|e| &e["document"]).collect();
                        if read.iter().collect::<Vec<_>>() != signed {
                            out.push(format!(
                                "{id}: the credentials read are not the ones signed"
                            ));
                        }
                    }
                    Err(e) => out.push(format!("{id}: the credentials: {e:?}")),
                }
            }
            other => out.push(format!("{id}: a case of kind {other:?}")),
        }
    }

    for (alg, _) in ALGORITHMS {
        for (kind, name) in [("vc", "credential"), ("vp", "presentation")] {
            if !covered.contains(&(kind, alg.to_owned())) {
                out.push(format!(
                    "coverage: the file holds no verified {name} signed under {alg}"
                ));
            }
        }
        if high_s::is_ecdsa(alg) {
            for kind in ["credential", "presentation"] {
                for (high, name) in [(true, "high-S"), (false, "low-S")] {
                    if signatures
                        .get(&(kind, alg.to_owned(), high))
                        .copied()
                        .unwrap_or(0)
                        == 0
                    {
                        out.push(format!(
                            "coverage: the file holds no {name} {alg} signature on a {kind}"
                        ));
                    }
                }
            }
        }
    }
    (out, signatures)
}

fn disagreements(file: &Value) -> Vec<String> {
    read_back(file).0
}

/// Counts the signature of `message` as high-S or low-S, and checks that the other `S` for the
/// same `R` verifies as well, by verifying the message with that signature in its place.
fn tally(
    id: &str,
    kind: &'static str,
    message: &[u8],
    key: &PublicKey,
    signatures: &mut Forms,
    out: &mut Vec<String>,
) {
    let alg = name_of(parts(message).alg);
    if !high_s::is_ecdsa(alg) {
        return;
    }
    let signature = signature(message);
    let high = high_s::is_high_s(alg, &signature);
    *signatures.entry((kind, alg.to_owned(), high)).or_default() += 1;
    let other = with_signature(message, &high_s::twin(alg, &signature));
    if verify_sign1(&other, key).is_err() {
        out.push(format!(
            "{id}: the other S for the same R does not verify, so {alg}'s order is wrong here"
        ));
    }
}

#[test]
fn what_an_independent_library_signed_reads_back_here() {
    let Some(given) = std::env::var_os("CHECK_COSE_ORACLE_ISSUED") else {
        return;
    };
    // Tests run in the crate directory, so a relative path is read from the workspace root,
    // where `cose_emit` writes and where the command is run from.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(given);
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let file: Value = serde_json::from_str(&text).expect("the file is JSON");
    let (wrong, signatures) = read_back(&file);
    assert!(
        wrong.is_empty(),
        "{} disagreement(s) with the library:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    let cases = file["cases"].as_array().expect("cases");
    let messages = cases.len()
        + cases
            .iter()
            .map(|c| c["envelopes"].as_array().map_or(0, Vec::len))
            .sum::<usize>();
    for ((kind, alg, high), n) in &signatures {
        eprintln!(
            "{alg}: {n} {} signatures on a {kind}",
            if *high { "high-S" } else { "low-S" }
        );
    }
    eprintln!(
        "verified {} cases, {messages} messages, with no disagreement",
        cases.len()
    );
}

/// A file of the shape the library writes, made of this crate's messages: each as it signs it,
/// low-S, and with `S` replaced by `n − S`, which is what a signer that does not normalise
/// writes about half the time.
fn selfmade() -> Value {
    use crate::jose::{sign_credential_cose, sign_presentation_cose};
    use crate::test_support::{cose_envelope, credential, presentation};

    let b64 = |bytes: &[u8]| base64::engine::general_purpose::STANDARD.encode(bytes);
    let mut cases = Vec::new();
    for (alg, _) in ALGORITHMS {
        let (key, kid) = (keys::signing(alg), keys::kid(alg));
        let signed = sign_credential_cose(&credential(), &key, &kid, &[]).expect("signs");
        let mut variants = vec![signed.clone()];
        if high_s::is_ecdsa(alg) {
            let twin = high_s::twin(alg, &signature(&signed));
            variants.push(with_signature(&signed, &twin));
        }
        for (n, variant) in variants.iter().enumerate() {
            cases.push(json!({
                "id": format!("{alg}/credential/{n}"), "kind": "vc", "alg": alg, "kid": kid,
                "message": b64(variant), "document": credential(),
            }));
        }
        // The first presentation is low-S throughout, and the second, where there is one, is
        // high-S throughout.
        for (n, variant) in variants.iter().enumerate() {
            let document = presentation(vec![cose_envelope(variant)]);
            let mut holder = sign_presentation_cose(&document, &key, &kid, &[]).expect("signs");
            if n == 1 {
                let twin = high_s::twin(alg, &signature(&holder));
                holder = with_signature(&holder, &twin);
            }
            cases.push(json!({
                "id": format!("{alg}/presentation/{n}"), "kind": "vp", "alg": alg, "kid": kid,
                "message": b64(&holder), "document": document,
                "envelopes": [{"index": 0, "kid": kid, "message": b64(variant), "document": credential()}],
            }));
        }
    }
    json!({ "keys": keys::published(), "cases": cases })
}

#[test]
fn the_readback_check_accepts_high_s_and_fails_on_a_mismatch() {
    let file = selfmade();
    assert_eq!(disagreements(&file), Vec::<String>::new(), "a clean file");

    // The document differs from what was signed.
    let mut altered = file.clone();
    altered["cases"][0]["document"]["id"] = json!("urn:uuid:other");
    assert!(
        disagreements(&altered).iter().any(|d| d.contains("read")),
        "a different document is found"
    );

    // A file that holds no high-S signature says nothing about high-S.
    let mut low_only = file.clone();
    low_only["cases"]
        .as_array_mut()
        .expect("cases")
        .retain(|c| c["id"].as_str().is_some_and(|id| !id.ends_with("/1")));
    let wrong = disagreements(&low_only);
    for alg in ["ES256", "ES384", "ES512"] {
        for kind in ["credential", "presentation"] {
            assert!(
                wrong
                    .iter()
                    .any(|d| d.contains(&format!("no high-S {alg} signature on a {kind}"))),
                "{alg} {kind}: {wrong:?}"
            );
        }
    }

    // High-S credentials do not stand in for high-S presentations: with only the high-S
    // presentations gone, each ECDSA algorithm's is named, and nothing else.
    let mut no_high_presentation = file.clone();
    no_high_presentation["cases"]
        .as_array_mut()
        .expect("cases")
        .retain(|c| {
            let id = c["id"].as_str().unwrap_or_default();
            !(c["kind"] == json!("vp") && id.ends_with("/1"))
        });
    let wrong = disagreements(&no_high_presentation);
    for alg in ["ES256", "ES384", "ES512"] {
        assert!(
            wrong
                .iter()
                .any(|d| d.contains(&format!("no high-S {alg} signature on a presentation"))),
            "{alg}: {wrong:?}"
        );
    }
    assert!(
        wrong.iter().all(|d| d.contains("on a presentation")),
        "only the presentations are named: {wrong:?}"
    );

    // The kid resolves to the key of another algorithm.
    let mut crossed = file.clone();
    let other = keys::kid("ES384");
    let index = crossed["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .position(|c| c["id"] == json!("ES256/credential/0"))
        .expect("a case");
    crossed["cases"][index]["kid"] = json!(other);
    assert!(
        !disagreements(&crossed).is_empty(),
        "a kid that names another key is found"
    );

    // A message with a changed byte of its signature is found.
    let mut tampered = file.clone();
    let text = tampered["cases"][0]["message"].as_str().expect("a message");
    let mut message = decode(text);
    let last = message.len() - 1;
    message[last] ^= 1;
    tampered["cases"][0]["message"] =
        json!(base64::engine::general_purpose::STANDARD.encode(message));
    assert!(
        !disagreements(&tampered).is_empty(),
        "a changed signature is found"
    );

    // A missing algorithm is a coverage failure, not a pass.
    let mut no_eddsa = file;
    no_eddsa["cases"]
        .as_array_mut()
        .expect("cases")
        .retain(|c| c["alg"] != json!("EdDSA"));
    let wrong = disagreements(&no_eddsa);
    for kind in ["credential", "presentation"] {
        assert!(
            wrong
                .iter()
                .any(|d| d.contains(&format!("no verified {kind} signed under EdDSA"))),
            "{kind}: {wrong:?}"
        );
    }
}

/// A file with no presentation does not show that presentations verify, so each algorithm's
/// missing presentation is named.
#[test]
fn a_file_without_presentations_names_each_missing_one() {
    let mut file = selfmade();
    file["cases"]
        .as_array_mut()
        .expect("cases")
        .retain(|c| c["kind"] == json!("vc"));
    let wrong = disagreements(&file);
    for (alg, _) in ALGORITHMS {
        assert!(
            wrong
                .iter()
                .any(|d| d.contains(&format!("no verified presentation signed under {alg}"))),
            "{alg}: {wrong:?}"
        );
    }
}
