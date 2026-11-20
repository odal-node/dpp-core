//! The `vc+sd-jwt` and `vp+sd-jwt` tokens this crate issues, for two independent
//! implementations of RFC 9901 to judge.
//!
//! ```text
//! EMIT_VC_SD_JWT_CORPUS=1 cargo test -p dpp-vc --lib jose::sd_jwt_oracle_tests
//! ```
//!
//! writes `target/sdjwt-oracle/vc-sd-jwt.json`, which `.github/oracle/sdjwt/vc-sd-jwt.mjs`
//! (the TypeScript library) and `VcSdJwtOracle.kt` (the EU wallet's Kotlin library) read.
//! Each case carries the token, the public key it was signed under, the document it was
//! made from, the paths that were concealed, the claims a verifier must read from the
//! token as presented, and what this crate reads. What the two libraries judge: the
//! signature under the published key, the Disclosures and their digests, `typ` and `cty`,
//! and the processed payload against the document.
//!
//! The cases include the three issuance inputs of the W3C suite (rows 17 to 19), issued
//! under the suite's own keys, so what the suite asks this crate to issue is what the
//! other implementations read.
//!
//! Without the variable the test still runs, and still checks that this crate reads every
//! token back to the claims it expects, so an ordinary `just check` gets that coverage
//! without producing files.

use std::path::Path;

use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::keystore::KeyEntry;
use dpp_crypto::sd_jwt::{ClaimPath, SdJwt};

use super::{
    sign_credential_sd_jwt, sign_presentation_sd_jwt, verify_credential_sd_jwt,
    verify_presentation_sd_jwt,
};
use crate::test_support::{KID, credential, generated, json, key_id, public_key, signing_key};

fn path(segments: &Value) -> ClaimPath {
    ClaimPath::from_json(segments).expect("a path")
}

fn paths(all: &Value) -> Vec<ClaimPath> {
    all.as_array().expect("an array").iter().map(path).collect()
}

/// `document` without the claims at `removed`, each a path in the JSON form. An array
/// element that is removed leaves no gap, as a verifier reads it. Paths are removed
/// from the last array position to the first, so removing one does not move the next.
fn without(document: &Value, removed: &[Value]) -> Value {
    let mut out = document.clone();
    let mut sorted: Vec<&Value> = removed.iter().collect();
    // The deepest first, so a claim is removed before the claim it sits in, and among paths
    // of one depth the last position first, so removing one does not move the next.
    sorted.sort_by(|a, b| {
        let depth = |p: &&Value| p.as_array().map_or(0, Vec::len);
        depth(b)
            .cmp(&depth(a))
            .then_with(|| b.to_string().cmp(&a.to_string()))
    });
    for p in sorted {
        let segments = p.as_array().expect("a path");
        let (last, parents) = segments.split_last().expect("not empty");
        let mut node = &mut out;
        for segment in parents {
            node = match segment {
                Value::String(name) => &mut node[name.as_str()],
                Value::Number(n) => &mut node[n.as_u64().expect("a position") as usize],
                _ => panic!("a path of names and positions"),
            };
        }
        match last {
            Value::String(name) => {
                node.as_object_mut().expect("an object").remove(name);
            }
            Value::Number(n) => {
                node.as_array_mut()
                    .expect("an array")
                    .remove(n.as_u64().expect("a position") as usize);
            }
            _ => panic!("a path of names and positions"),
        }
    }
    out
}

/// One thing a signer issued, and what a verifier of each part must read.
struct Issued {
    id: &'static str,
    presentation: bool,
    key: KeyEntry,
    kid: String,
    document: Value,
    concealed: Vec<Value>,
    sd_jwt: SdJwt,
}

impl Issued {
    fn credential(
        id: &'static str,
        key: KeyEntry,
        kid: String,
        document: Value,
        concealed: &Value,
    ) -> Self {
        let sd_jwt =
            sign_credential_sd_jwt(&document, &key, &kid, &[], &paths(concealed)).expect("issued");
        Self {
            id,
            presentation: false,
            key,
            kid,
            document,
            concealed: concealed.as_array().expect("an array").clone(),
            sd_jwt,
        }
    }

    fn presentation(
        id: &'static str,
        key: KeyEntry,
        kid: String,
        document: Value,
        concealed: &Value,
    ) -> Self {
        let sd_jwt = sign_presentation_sd_jwt(&document, &key, &kid, &[], &paths(concealed))
            .expect("issued");
        Self {
            id,
            presentation: true,
            key,
            kid,
            document,
            concealed: concealed.as_array().expect("an array").clone(),
            sd_jwt,
        }
    }

    /// The token presenting `reveal`, which is a list of paths, and the claims a verifier
    /// must read from it: the document without every concealed claim that is not revealed.
    fn presented(&self, reveal: &Value) -> (SdJwt, Value) {
        let wanted = paths(reveal);
        let digests = self
            .sd_jwt
            .digests_for_paths(&wanted)
            .expect("the paths are in the token");
        let refs: Vec<&str> = digests.iter().map(String::as_str).collect();
        let token = self.sd_jwt.present(&refs).expect("presents");
        // Revealing a claim reveals the claims it sits in, so a concealed claim stays hidden
        // only when no revealed path runs through it.
        let revealed = reveal.as_array().expect("an array");
        let hidden: Vec<Value> = self
            .concealed
            .iter()
            .filter(|c| {
                let c = c.as_array().expect("a path");
                !revealed
                    .iter()
                    .any(|r| r.as_array().expect("a path").starts_with(c))
            })
            .cloned()
            .collect();
        (token, without(&self.document, &hidden))
    }

    /// What this crate reads from `token`.
    fn ours(&self, token: &SdJwt) -> Value {
        let public = self.key.public_key();
        let text = token.serialise();
        let read = if self.presentation {
            verify_presentation_sd_jwt(&text, &public, None).map(|p| p.payload().clone())
        } else {
            verify_credential_sd_jwt(&text, &public, None)
        };
        match read {
            Ok(payload) => json!({"accepts": true, "payload": payload}),
            Err(e) => json!({"accepts": false, "error": e.to_string()}),
        }
    }

    fn case(&self, label: &str, token: &SdJwt, expected: &Value) -> Value {
        let (typ, cty) = if self.presentation {
            ("vp+sd-jwt", "vp")
        } else {
            ("vc+sd-jwt", "vc")
        };
        json!({
            "id": format!("{}/{label}", self.id),
            "kind": if self.presentation { "presentation" } else { "credential" },
            "token": token.serialise(),
            "jwk": self.key.public_key().jwk(),
            "alg": self.key.algorithm().jose_alg(),
            "kid": self.kid,
            "typ": typ,
            "cty": cty,
            "document": self.document,
            "concealed": self.concealed,
            "expected": expected,
            "ours": self.ours(token),
        })
    }

    /// The token as issued, with every Disclosure, and any partial presentations asked for.
    fn cases(&self, partial: &[(&str, Value)]) -> Vec<Value> {
        let mut cases = vec![self.case("issued", &self.sd_jwt, &self.document)];
        for (label, reveal) in partial {
            let (token, expected) = self.presented(reveal);
            cases.push(self.case(label, &token, &expected));
        }
        cases
    }
}

fn issued() -> Vec<Issued> {
    let mut rows = Vec::new();

    // Rows 17 to 19 of the W3C suite, under the suite's keys and its `kid`.
    rows.push(Issued::credential(
        "suite/row-17",
        signing_key("p384"),
        key_id("p384"),
        json_file("credential-selective.json"),
        &json!([
            ["credentialSubject", "firstName"],
            ["credentialSubject", "lastName"]
        ]),
    ));
    rows.push(Issued::credential(
        "suite/row-18",
        signing_key("p521"),
        key_id("p521"),
        json_file("credential-nested-selective.json"),
        &json!([
            ["credentialSubject", "address", "street"],
            ["credentialSubject", "address", "city"],
            ["credentialSubject", "phoneNumbers", 0]
        ]),
    ));
    rows.push(Issued::presentation(
        "suite/row-19",
        signing_key("p384"),
        key_id("p384"),
        json_file("presentation-selective.json"),
        &json!([["holder"], ["verifiableCredential", 0]]),
    ));

    // The same mechanism under the other two algorithms, and the shapes the suite does
    // not use: a Disclosure inside a Disclosure, every element of an array, and nothing
    // concealed at all.
    let mut nested = credential();
    nested["credentialSubject"] = json!({
        "id": "did:example:subject",
        "name": "Example",
        "address": {"street": "1 Main St", "city": "Anytown"},
        "phones": ["+1-555-0001", "+1-555-0002", "+1-555-0003"],
    });
    rows.push(Issued::credential(
        "ed25519/nested",
        generated(KeyAlgorithm::Ed25519),
        KID.to_owned(),
        nested.clone(),
        &json!([
            ["credentialSubject", "address"],
            ["credentialSubject", "address", "street"],
            ["credentialSubject", "name"]
        ]),
    ));
    rows.push(Issued::credential(
        "es256/every-element",
        generated(KeyAlgorithm::EcdsaP256),
        KID.to_owned(),
        nested,
        &json!([
            ["credentialSubject", "phones", 0],
            ["credentialSubject", "phones", 1],
            ["credentialSubject", "phones", 2]
        ]),
    ));
    rows.push(Issued::credential(
        "ed25519/nothing-concealed",
        generated(KeyAlgorithm::Ed25519),
        KID.to_owned(),
        credential(),
        &json!([]),
    ));
    rows
}

fn json_file(name: &str) -> Value {
    json(name)
}

/// Every case: each token as issued, and the presentations that reveal part of it.
fn cases() -> Vec<Value> {
    let mut cases = Vec::new();
    for row in issued() {
        let partial: Vec<(&str, Value)> = match row.id {
            "suite/row-17" => vec![
                (
                    "only-firstName",
                    json!([["credentialSubject", "firstName"]]),
                ),
                ("nothing", json!([])),
            ],
            "suite/row-18" => vec![
                (
                    "only-street",
                    json!([["credentialSubject", "address", "street"]]),
                ),
                (
                    "only-the-first-phone",
                    json!([["credentialSubject", "phoneNumbers", 0]]),
                ),
                ("nothing", json!([])),
            ],
            "suite/row-19" => vec![
                ("nothing", json!([])),
                ("only-the-holder", json!([["holder"]])),
            ],
            "ed25519/nested" => vec![
                (
                    "the-address-without-its-street",
                    json!([["credentialSubject", "address"]]),
                ),
                (
                    "the-street-with-its-address",
                    json!([["credentialSubject", "address", "street"]]),
                ),
                ("nothing", json!([])),
            ],
            "es256/every-element" => vec![
                (
                    "only-the-second",
                    json!([["credentialSubject", "phones", 1]]),
                ),
                ("nothing", json!([])),
            ],
            _ => vec![],
        };
        cases.extend(row.cases(&partial));
    }
    cases
}

#[test]
fn this_crate_reads_every_token_it_issues_back_to_the_claims_expected() {
    let cases = cases();

    assert_eq!(cases.len(), 18, "the cases that are issued and presented");
    for case in &cases {
        let id = &case["id"];
        assert_eq!(case["ours"]["accepts"], true, "{id}: {}", case["ours"]);
        assert_eq!(case["ours"]["payload"], case["expected"], "{id}");
    }
    let ids: Vec<&str> = cases.iter().filter_map(|c| c["id"].as_str()).collect();
    for row in [
        "suite/row-17/issued",
        "suite/row-18/issued",
        "suite/row-19/issued",
    ] {
        assert!(ids.contains(&row), "{row}");
    }
    assert!(
        public_key("p384").algorithm() == KeyAlgorithm::EcdsaP384,
        "the suite's P-384 key"
    );
}

#[test]
fn emit() {
    let cases = cases();
    if std::env::var_os("EMIT_VC_SD_JWT_CORPUS").is_none() {
        return;
    }
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/sdjwt-oracle");
    std::fs::create_dir_all(&dir).expect("create the oracle output directory");
    let body = json!({"cases": cases});
    std::fs::write(
        dir.join("vc-sd-jwt.json"),
        serde_json::to_string(&body).expect("serialises"),
    )
    .expect("write the corpus");
    eprintln!("wrote {} cases to {}", cases.len(), dir.display());
}
