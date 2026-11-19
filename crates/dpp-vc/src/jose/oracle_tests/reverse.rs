//! Tokens an independent JOSE library signed, verified by this crate.
//!
//! The other direction of the oracle. `.github/oracle/jose/issue.mjs` signs
//! credentials and presentations with the library, under EdDSA, ES256, ES384 and
//! ES512 and the suite's own keys. This crate verifies each token with
//! [`verify_credential`] and [`verify_presentation`], and the credentials a
//! presentation envelopes with [`VerifiedPresentation::verify_credentials`](crate::jose::VerifiedPresentation::verify_credentials),
//! and has to arrive at the document the library signed.
//!
//! This crate signs the lower `S` of an ECDSA signature and verifies both. Node's
//! ECDSA does not normalise, so the library's output holds high-S signatures, and
//! this check holds the file to it: each ECDSA algorithm must have at least one
//! high-S signature and at least one low-S one, counted here from the signatures
//! and not from the file's say-so, and the low-S twin of each high-S signature must
//! verify, which shows the curve order used to tell them apart is the right one.
//! Without that a file the library had signed with low-S only would pass, and the
//! claim that high-S is verified would be vacuous.
//!
//! ```text
//! CHECK_JOSE_ORACLE_ISSUED=target/jose-oracle/issued.json \
//!   cargo test -p dpp-vc --lib jose::oracle_tests::reverse
//! ```
//!
//! Without the variable the first test does nothing, because there is no file to
//! read. The second runs regardless: it builds a file of the same shape from this
//! crate's own tokens and their high-S twins, and shows that the check accepts it
//! and fails on each way a file can be wrong, so a green run of the first means
//! something.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::keys::{self, ALGORITHMS};
use super::{high_s, token};
use crate::jose::{verify_credential, verify_presentation};
use dpp_crypto::jws::PublicKey;

/// Every way `file` disagrees with this crate, and how many signatures of each form
/// (algorithm, high-S) it holds.
///
/// `file` is `{ "keys": { <kid>: <public JWK> }, "cases": [...] }`. A case is
/// `{ "id", "kind": "vc" | "vp", "alg", "kid", "token", "document", "now"?,
/// "envelopes"? }`, where `document` is what the library signed, `now` is the
/// instant a Verifier checks `exp` and `nbf` at, and `envelopes` lists the
/// credentials of a presentation as `{ "index", "kid", "token", "document" }`.
fn read_back(file: &Value) -> (Vec<String>, BTreeMap<(String, bool), usize>) {
    let keys = file["keys"].as_object().expect("the file lists its keys");
    let key_of = |kid: &str| keys.get(kid).and_then(PublicKey::from_jwk);
    let cases = file["cases"].as_array().expect("the file holds cases");
    let mut out = Vec::new();
    let mut signatures: BTreeMap<(String, bool), usize> = BTreeMap::new();
    let mut per_alg: BTreeMap<String, usize> = BTreeMap::new();

    for case in cases {
        let id = case["id"].as_str().unwrap_or("?");
        let token = case["token"].as_str().expect("a case has a token");
        let kid = case["kid"].as_str().expect("a case has a kid");
        let Some(key) = key_of(kid) else {
            out.push(format!("{id}: no key is published for the kid {kid}"));
            continue;
        };
        let now = case["now"].as_i64();
        *per_alg
            .entry(case["alg"].as_str().unwrap_or("?").to_owned())
            .or_default() += 1;
        tally(id, token, &key, &mut signatures, &mut out);

        match case["kind"].as_str() {
            Some("vc") => match verify_credential(token, &key, now) {
                Ok(document) if document == case["document"] => {}
                Ok(document) => out.push(format!(
                    "{id}: read {document}, the library signed {}",
                    case["document"]
                )),
                Err(e) => out.push(format!("{id}: {e:?}")),
            },
            Some("vp") => {
                let verified = match verify_presentation(token, &key, now) {
                    Ok(verified) => verified,
                    Err(e) => {
                        out.push(format!("{id}: {e:?}"));
                        continue;
                    }
                };
                if verified.payload() != &case["document"] {
                    out.push(format!(
                        "{id}: read {}, the library signed {}",
                        verified.payload(),
                        case["document"]
                    ));
                }
                let listed = case["envelopes"].as_array().cloned().unwrap_or_default();
                let in_payload = token::envelope_tokens(verified.payload());
                for envelope in &listed {
                    let index = usize::try_from(envelope["index"].as_u64().unwrap_or(u64::MAX))
                        .unwrap_or(usize::MAX);
                    let inner = envelope["token"].as_str().expect("an envelope token");
                    if !in_payload.contains(&(index, inner.to_owned())) {
                        out.push(format!("{id}: envelope {index} is not the one listed"));
                    }
                    let inner_kid = envelope["kid"].as_str().expect("an envelope kid");
                    match key_of(inner_kid) {
                        Some(inner_key) => {
                            tally(id, inner, &inner_key, &mut signatures, &mut out);
                        }
                        None => out.push(format!("{id}: no key for the kid {inner_kid}")),
                    }
                }
                match verified.verify_credentials(now, |e| key_of(e.kid.as_deref()?)) {
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
        if per_alg.get(alg).copied().unwrap_or(0) == 0 {
            out.push(format!(
                "coverage: the file holds no token signed under {alg}"
            ));
        }
        if high_s::is_ecdsa(alg) {
            for (high, name) in [(true, "high-S"), (false, "low-S")] {
                if signatures
                    .get(&(alg.to_owned(), high))
                    .copied()
                    .unwrap_or(0)
                    == 0
                {
                    out.push(format!(
                        "coverage: the file holds no {name} {alg} signature"
                    ));
                }
            }
        }
    }
    (out, signatures)
}

fn disagreements(file: &Value) -> Vec<String> {
    read_back(file).0
}

/// Counts the signature of `token` as high-S or low-S, and checks that the other
/// `S` for the same `R` verifies as well.
fn tally(
    id: &str,
    token: &str,
    key: &PublicKey,
    signatures: &mut BTreeMap<(String, bool), usize>,
    out: &mut Vec<String>,
) {
    let alg = token::header(token)["alg"]
        .as_str()
        .unwrap_or("?")
        .to_owned();
    if !high_s::is_ecdsa(&alg) {
        return;
    }
    let signature = token::signature(token);
    let high = high_s::is_high_s(&alg, &signature);
    *signatures.entry((alg.clone(), high)).or_default() += 1;
    let (input, _) = token.rsplit_once('.').expect("a compact JWS");
    if !key.verify(input.as_bytes(), &high_s::twin(&alg, &signature)) {
        out.push(format!(
            "{id}: the other S for the same R does not verify, so {alg}'s order is wrong here"
        ));
    }
}

#[test]
fn what_an_independent_library_signed_reads_back_here() {
    let Some(given) = std::env::var_os("CHECK_JOSE_ORACLE_ISSUED") else {
        return;
    };
    // Tests run in the crate directory, so a relative path is read from the
    // workspace root, where `emit` writes and where the command is run from.
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
    let tokens = cases.len()
        + cases
            .iter()
            .map(|c| c["envelopes"].as_array().map_or(0, Vec::len))
            .sum::<usize>();
    for ((alg, high), n) in &signatures {
        eprintln!(
            "{alg}: {n} {} signatures",
            if *high { "high-S" } else { "low-S" }
        );
    }
    eprintln!(
        "verified {} cases, {tokens} tokens, with no disagreement",
        cases.len()
    );
}

/// A file of the shape the library writes, made of this crate's tokens: each as it
/// signs it, low-S, and with `S` replaced by `n − S`, which is what a signer that
/// does not normalise writes about half the time.
fn selfmade() -> Value {
    use crate::jose::{sign_credential, sign_presentation};
    use crate::test_support::{credential, envelope, presentation};

    let mut cases = Vec::new();
    for (alg, _) in ALGORITHMS {
        let (key, kid) = (keys::signing(alg), keys::kid(alg));
        let signed = sign_credential(&credential(), &key, &kid, &[]).expect("signs");
        let mut variants = vec![signed.clone()];
        if high_s::is_ecdsa(alg) {
            let twin = high_s::twin(alg, &token::signature(&signed));
            variants.push(token::with_signature(&signed, &twin));
        }
        for (n, variant) in variants.iter().enumerate() {
            cases.push(json!({
                "id": format!("{alg}/credential/{n}"), "kind": "vc", "alg": alg, "kid": kid,
                "token": variant, "document": credential(),
            }));
        }
        // The first presentation is low-S throughout, and the second, where there is
        // one, is high-S throughout.
        for (n, variant) in variants.iter().enumerate() {
            let document = presentation(vec![envelope(variant)]);
            let mut holder = sign_presentation(&document, &key, &kid, &[]).expect("signs");
            if n == 1 {
                let twin = high_s::twin(alg, &token::signature(&holder));
                holder = token::with_signature(&holder, &twin);
            }
            cases.push(json!({
                "id": format!("{alg}/presentation/{n}"), "kind": "vp", "alg": alg, "kid": kid,
                "token": holder, "document": document,
                "envelopes": [{"index": 0, "kid": kid, "token": variant, "document": credential()}],
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
        assert!(
            wrong
                .iter()
                .any(|d| d.contains(&format!("no high-S {alg} signature"))),
            "{alg}: {wrong:?}"
        );
    }

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

    // A missing algorithm is a coverage failure, not a pass.
    let mut no_eddsa = file;
    no_eddsa["cases"]
        .as_array_mut()
        .expect("cases")
        .retain(|c| c["alg"] != json!("EdDSA"));
    assert!(
        disagreements(&no_eddsa)
            .iter()
            .any(|d| d.contains("no token signed under EdDSA"))
    );
}
