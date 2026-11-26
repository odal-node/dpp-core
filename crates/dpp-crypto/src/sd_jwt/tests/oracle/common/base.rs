//! The small credential the adversarial cases mutate, and the helpers that build one case.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use serde_json::{Value, json};

use crate::sd_jwt::digest_of;

use super::plan::{Issued, issue_to_plan, path, paths};
use super::signing::Issuer;

/// A small credential with every kind of Disclosure in it: two properties, a
/// property inside an object, and an array element.
pub(crate) struct Base<'a> {
    pub(crate) issuer: &'a Issuer,
    pub(crate) issued: Issued,
}

impl<'a> Base<'a> {
    pub(crate) fn new(issuer: &'a Issuer) -> Self {
        let claims = json!({
            "iss": "https://issuer.example",
            "given_name": "Alice",
            "family_name": "Doe",
            "address": {"city": "Berlin", "zip": "10115"},
            "nat": ["DE", "FR", "IT"]
        });
        let hidden = paths(&[
            &["given_name"],
            &["family_name"],
            &["address", "city"],
            &["nat", "#1"],
        ]);
        Self {
            issuer,
            issued: issue_to_plan(&claims, &hidden),
        }
    }

    pub(crate) fn payload(&self) -> Value {
        self.issued.payload.clone()
    }

    pub(crate) fn disclosures(&self) -> Vec<String> {
        self.issued
            .disclosures
            .iter()
            .map(|d| d.encoded().to_owned())
            .collect()
    }

    pub(crate) fn digest(&self, spec: &[&str]) -> String {
        self.issued.digest_at[&path(spec)].clone()
    }

    pub(crate) fn disclosure(&self, spec: &[&str]) -> String {
        let digest = self.digest(spec);
        self.issued
            .disclosures
            .iter()
            .find(|d| d.digest() == digest)
            .expect("the plan hid this path")
            .encoded()
            .to_owned()
    }

    pub(crate) fn token(&self, payload: &Value, disclosures: &[String]) -> String {
        self.issuer.token(payload, disclosures)
    }

    pub(crate) fn plain(&self) -> String {
        self.token(&self.payload(), &self.disclosures())
    }

    /// The valid token with one change applied to its payload and Disclosures.
    pub(crate) fn altered(&self, change: impl FnOnce(&mut Value, &mut Vec<String>)) -> String {
        let (mut payload, mut disclosures) = (self.payload(), self.disclosures());
        change(&mut payload, &mut disclosures);
        self.token(&payload, &disclosures)
    }
}

/// Where the placeholder for `digest` sits in the `nat` array.
///
/// The issuer puts decoy placeholders among the elements, so the one that stands for
/// an element is not at the position of the element it hides.
pub(crate) fn nat_placeholder(payload: &Value, digest: &str) -> usize {
    payload["nat"]
        .as_array()
        .expect("the payload has nat")
        .iter()
        .position(|item| item["..."] == json!(digest))
        .expect("the placeholder is in nat")
}

/// A Disclosure written by hand: its encoding and its SHA-256 digest.
pub(crate) fn raw(json_text: &str) -> (String, String) {
    let encoded = B64.encode(json_text);
    let digest = digest_of(&encoded);
    (encoded, digest)
}

pub(crate) fn sd(payload: &mut Value) -> &mut Vec<Value> {
    payload["_sd"].as_array_mut().expect("the payload has _sd")
}

pub(crate) fn add(out: &mut Vec<Adversarial>, id: &str, clause: &'static str, token: String) {
    out.push(Adversarial {
        id: id.to_owned(),
        clause,
        token,
    });
}

/// A hand-written Disclosure, referenced from the root `_sd`.
pub(crate) fn with_disclosure(base: &Base<'_>, json_text: &str) -> String {
    base.altered(|payload, ds| {
        let (encoded, digest) = raw(json_text);
        sd(payload).push(json!(digest));
        ds.push(encoded);
    })
}

pub(crate) struct Adversarial {
    pub(crate) id: String,
    /// The clause of RFC 9901 the case turns on, or why it is an edge.
    pub(crate) clause: &'static str,
    pub(crate) token: String,
}
