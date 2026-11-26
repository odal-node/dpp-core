//! The corpus the oracle emits and the check that this crate reads it back.

use std::path::Path;

use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::keystore::KeyEntry;
use dpp_crypto::sd_jwt::{ClaimPath, SdJwt, holder_key_confirmation};

use super::super::super::super::VerifyOptions;
use super::super::super::super::{
    KeyBindingRequirement, sign_credential_sd_jwt, sign_presentation_sd_jwt,
    verify_credential_sd_jwt, verify_presentation_sd_jwt,
};
use crate::test_support::NOT_REQUIRED;
use crate::test_support::{KID, credential, generated, json, key_id, public_key, signing_key};

use super::bound::{NOW, issued_to_holders, policy};

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
pub(super) struct Issued {
    pub(super) id: &'static str,
    pub(super) presentation: bool,
    pub(super) key: KeyEntry,
    pub(super) kid: String,
    pub(super) document: Value,
    pub(super) concealed: Vec<Value>,
    pub(super) sd_jwt: SdJwt,
    /// The Holder whose key the token names in `cnf`, when it was issued to one. `document`
    /// then holds that `cnf`, since a verifier reads it.
    pub(super) holder: Option<KeyEntry>,
}

impl Issued {
    fn credential(
        id: &'static str,
        key: KeyEntry,
        kid: String,
        document: Value,
        concealed: &Value,
    ) -> Self {
        Self::issue(id, false, key, kid, document, concealed, None)
    }

    fn presentation(
        id: &'static str,
        key: KeyEntry,
        kid: String,
        document: Value,
        concealed: &Value,
    ) -> Self {
        Self::issue(id, true, key, kid, document, concealed, None)
    }

    pub(super) fn issue(
        id: &'static str,
        presentation: bool,
        key: KeyEntry,
        kid: String,
        mut document: Value,
        concealed: &Value,
        holder: Option<KeyEntry>,
    ) -> Self {
        let public = holder.as_ref().map(KeyEntry::public_key);
        let claims = paths(concealed);
        let sd_jwt = if presentation {
            sign_presentation_sd_jwt(&document, &key, &kid, &[], &claims, public.as_ref())
        } else {
            sign_credential_sd_jwt(&document, &key, &kid, &[], &claims, public.as_ref())
        }
        .expect("issued");
        if let Some(public) = &public {
            document["cnf"] = holder_key_confirmation(public);
        }
        Self {
            id,
            presentation,
            key,
            kid,
            document,
            concealed: concealed.as_array().expect("an array").clone(),
            sd_jwt,
            holder,
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

    /// What this crate reads from `token`, with Key Binding required when `bound`, at the
    /// policy of the challenge.
    fn ours(&self, token: &SdJwt, bound: bool) -> Value {
        let public = self.key.public_key();
        let text = token.serialise();
        let (options, binding) = if bound {
            (
                VerifyOptions::new().at(NOW),
                KeyBindingRequirement::Required(policy()),
            )
        } else {
            (VerifyOptions::new(), NOT_REQUIRED)
        };
        let read = if self.presentation {
            verify_presentation_sd_jwt(&text, &public, options, &binding)
                .map(|p| p.payload().clone())
        } else {
            verify_credential_sd_jwt(&text, &public, options, &binding)
        };
        match read {
            Ok(payload) => json!({"accepts": true, "payload": payload}),
            Err(e) => json!({"accepts": false, "error": e.to_string()}),
        }
    }

    fn case(&self, label: &str, token: &SdJwt, expected: &Value) -> Value {
        self.case_of(label, token, expected, false)
    }

    pub(super) fn case_of(
        &self,
        label: &str,
        token: &SdJwt,
        expected: &Value,
        bound: bool,
    ) -> Value {
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
            "ours": self.ours(token, bound),
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
    for row in issued_to_holders() {
        cases.extend(row.bound_cases());
    }
    cases
}

#[test]
fn this_crate_reads_every_token_it_issues_back_to_the_claims_expected() {
    let cases = cases();

    assert_eq!(
        cases.len(),
        25,
        "the cases that are issued, presented and bound"
    );
    for case in &cases {
        let id = &case["id"];
        // A case with no `accepts` is one every reader must accept.
        let accepts = case.get("accepts").unwrap_or(&json!(true)).clone();
        assert_eq!(case["ours"]["accepts"], accepts, "{id}: {}", case["ours"]);
        if accepts == json!(true) {
            assert_eq!(case["ours"]["payload"], case["expected"], "{id}");
        }
    }
    let ids: Vec<&str> = cases.iter().filter_map(|c| c["id"].as_str()).collect();
    for row in [
        "suite/row-17/issued",
        "suite/row-18/issued",
        "suite/row-19/issued",
    ] {
        assert!(ids.contains(&row), "{row}");
    }
    assert_eq!(
        public_key("p384").algorithm(),
        KeyAlgorithm::EcdsaP384,
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
