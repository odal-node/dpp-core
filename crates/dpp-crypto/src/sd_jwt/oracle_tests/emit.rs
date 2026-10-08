//! Build the corpus, check this crate against its own plan, and write it out.
//!
//! ```text
//! EMIT_SDJWT_CORPUS=1 cargo test -p dpp-crypto --lib sd_jwt::oracle_tests::emit
//! ```
//!
//! Writes `target/sdjwt-oracle/corpus.json`, which `.github/oracle/sdjwt/judge.mjs`
//! reads. Without the variable the test still runs and still checks that the
//! corpus is complete and that this crate reads every token it issued back to the
//! claims it was given, so an ordinary `just check` gets that coverage without
//! producing files.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::{Value, json};

use crate::sd_jwt::SdJwt;

use super::adversarial;
use super::issued::{self, show};
use super::plan::{self, Rng, issue_to_plan};
use super::rfc;
use super::signing::{Issuer, ours, structure};

/// A hand-listed plan this small is presented every way; a larger one is sampled.
const ENUMERATE_UP_TO: usize = 8;
const SAMPLES: usize = 24;

struct Corpus {
    cases: Vec<Value>,
    /// Tokens built from a plan, with the claims a verifier must see.
    planned: Vec<(String, Value, Value)>,
}

fn build(issuer: &Issuer) -> Corpus {
    let mut corpus = Corpus {
        cases: Vec::new(),
        planned: Vec::new(),
    };
    for case in issued::cases() {
        let issued = issue_to_plan(&case.claims, &case.hidden);
        let jwt = issuer.sign(&issued.payload);
        let sd_jwt = SdJwt::new(jwt, issued.disclosures.clone());

        let sets: Vec<BTreeSet<plan::Path>> = if case.hidden.len() <= ENUMERATE_UP_TO {
            plan::every_presentation(&case.hidden)
        } else {
            let seed = case
                .id
                .bytes()
                .fold(7u64, |h, b| h.wrapping_mul(31).wrapping_add(u64::from(b)));
            let mut rng = Rng(seed);
            plan::sampled_presentations(&case.hidden, &mut rng, SAMPLES)
        };
        for (n, presented) in sets.iter().enumerate() {
            let digests: Vec<&str> = presented
                .iter()
                .map(|p| issued.digest_at[p].as_str())
                .collect();
            let token = sd_jwt.present(&digests).serialise();
            let expected = plan::reveal(&case.claims, &case.hidden, presented);
            let everything = presented.len() == case.hidden.len();
            let id = if everything {
                format!("issued/{}", case.id)
            } else {
                format!("presented/{}/{n}", case.id)
            };
            corpus.cases.push(json!({
                "id": id,
                "group": if everything { "issued" } else { "presented" },
                "token": token,
                "presented": presented.iter().map(show).collect::<Vec<_>>(),
                "hiddenCount": case.hidden.len(),
                "expected": expected,
                "ours": ours(&token, &issuer.public_key_b64).to_json(),
            }));
            corpus.planned.push((id, expected, Value::String(token)));
        }
    }

    for case in adversarial::cases(issuer) {
        corpus.cases.push(json!({
            "id": format!("adversarial/{}", case.id),
            "group": "adversarial",
            "clause": case.clause,
            "token": case.token,
            "ours": ours(&case.token, &issuer.public_key_b64).to_json(),
        }));
    }

    for case in rfc::cases() {
        corpus.cases.push(json!({
            "id": format!("rfc/{}", case.id),
            "group": "rfc",
            "token": case.token,
            "printed": case.printed,
            "ours": structure(case.token).to_json(),
        }));
    }
    corpus
}

fn count(corpus: &Corpus, group: &str) -> usize {
    corpus
        .cases
        .iter()
        .filter(|c| c["group"] == json!(group))
        .count()
}

#[test]
fn the_corpus_is_complete_and_this_crate_reads_every_token_it_issued_back_to_the_plan() {
    let issuer = Issuer::new();
    let corpus = build(&issuer);

    // This crate against a plan it did not produce. A token this crate issued
    // and cannot read back is a defect whether or not anything else judges it.
    let mut disagreements = Vec::new();
    for (id, expected, token) in &corpus.planned {
        let verdict = ours(token.as_str().expect("a string"), &issuer.public_key_b64);
        if verdict.payload.as_ref() != Some(expected) {
            disagreements.push(format!("{id}: {verdict:?}"));
        }
    }
    assert!(
        disagreements.is_empty(),
        "{} issued token(s) do not read back to the claims they were issued from:\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );

    // Controls are the cases that make "both refused" mean something.
    for case in corpus.cases.iter().filter(|c| {
        c["id"]
            .as_str()
            .is_some_and(|id| id.starts_with("adversarial/control/"))
    }) {
        assert_eq!(
            case["ours"]["accepts"],
            json!(true),
            "a control must be accepted: {}",
            case["id"]
        );
    }

    // A count, because a loop over an empty corpus passes.
    assert!(count(&corpus, "issued") >= 40, "issued");
    assert!(count(&corpus, "presented") >= 500, "presented");
    assert!(count(&corpus, "adversarial") >= 50, "adversarial");
    assert_eq!(count(&corpus, "rfc"), 7, "the RFC's tokens");

    if std::env::var_os("EMIT_SDJWT_CORPUS").is_none() {
        return;
    }
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/sdjwt-oracle");
    std::fs::create_dir_all(&dir).expect("create the oracle output directory");
    let body = json!({
        "issuerJwk": issuer.jwk(),
        "kid": issuer.kid(),
        "cases": corpus.cases,
    });
    std::fs::write(
        dir.join("corpus.json"),
        serde_json::to_string(&body).expect("serialises"),
    )
    .expect("write the corpus");
    eprintln!(
        "wrote {} cases ({} issued, {} presented, {} adversarial, {} RFC) to {}",
        corpus.cases.len(),
        count(&corpus, "issued"),
        count(&corpus, "presented"),
        count(&corpus, "adversarial"),
        count(&corpus, "rfc"),
        dir.display()
    );
}
