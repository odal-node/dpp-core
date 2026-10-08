//! Tokens this crate should refuse, controls it should accept, and the edge cases
//! on which two implementations could legitimately differ.
//!
//! Every token is signed, including the broken ones. A mutation that left a
//! stale signature would be refused by any verifier for that reason alone, and
//! the question here is what each makes of the structure. So the payload is
//! altered first and signed afterwards, and the only defect is the one named.
//!
//! The case carries no verdict. The verdicts are this crate's and the other
//! implementation's, and the corpus emitter and the judge each record theirs.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha512};

use crate::sd_jwt::digest_of;

use super::plan::{Issued, Path, issue_to_plan, path, paths};
use super::signing::{Issuer, assemble};

pub(super) struct Adversarial {
    pub(super) id: String,
    /// The clause of RFC 9901 the case turns on, or why it is an edge.
    pub(super) clause: &'static str,
    pub(super) token: String,
}

/// A small credential with every kind of Disclosure in it: two properties, a
/// property inside an object, and an array element.
struct Base<'a> {
    issuer: &'a Issuer,
    issued: Issued,
}

impl<'a> Base<'a> {
    fn new(issuer: &'a Issuer) -> Self {
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

    fn payload(&self) -> Value {
        self.issued.payload.clone()
    }

    fn disclosures(&self) -> Vec<String> {
        self.issued
            .disclosures
            .iter()
            .map(|d| d.encoded().to_owned())
            .collect()
    }

    fn digest(&self, spec: &[&str]) -> String {
        self.issued.digest_at[&path(spec)].clone()
    }

    fn disclosure(&self, spec: &[&str]) -> String {
        let digest = self.digest(spec);
        self.issued
            .disclosures
            .iter()
            .find(|d| d.digest() == digest)
            .expect("the plan hid this path")
            .encoded()
            .to_owned()
    }

    fn token(&self, payload: &Value, disclosures: &[String]) -> String {
        self.issuer.token(payload, disclosures)
    }

    fn plain(&self) -> String {
        self.token(&self.payload(), &self.disclosures())
    }

    /// The valid token with one change applied to its payload and Disclosures.
    fn altered(&self, change: impl FnOnce(&mut Value, &mut Vec<String>)) -> String {
        let (mut payload, mut disclosures) = (self.payload(), self.disclosures());
        change(&mut payload, &mut disclosures);
        self.token(&payload, &disclosures)
    }
}

/// A Disclosure written by hand: its encoding and its SHA-256 digest.
fn raw(json_text: &str) -> (String, String) {
    let encoded = B64.encode(json_text);
    let digest = digest_of(&encoded);
    (encoded, digest)
}

fn sd(payload: &mut Value) -> &mut Vec<Value> {
    payload["_sd"].as_array_mut().expect("the payload has _sd")
}

fn add(out: &mut Vec<Adversarial>, id: &str, clause: &'static str, token: String) {
    out.push(Adversarial {
        id: id.to_owned(),
        clause,
        token,
    });
}

/// A hand-written Disclosure, referenced from the root `_sd`.
fn with_disclosure(base: &Base<'_>, json_text: &str) -> String {
    base.altered(|payload, ds| {
        let (encoded, digest) = raw(json_text);
        sd(payload).push(json!(digest));
        ds.push(encoded);
    })
}

pub(super) fn cases(issuer: &Issuer) -> Vec<Adversarial> {
    let base = Base::new(issuer);
    let mut out = Vec::new();
    controls(&base, &mut out);
    wrong_structure(&base, &mut out);
    malformed_disclosures(&base, &mut out);
    malformed_serialisation(&base, &mut out);
    edges(&base, &mut out);
    out
}

/// Tokens both must accept, so that "both refuse" is not what every case shows.
fn controls(base: &Base<'_>, out: &mut Vec<Adversarial>) {
    add(out, "control/valid", "clause 7.1", base.plain());
    add(
        out,
        "control/disclosures-reordered",
        "clause 7.1 puts no order on the Disclosures",
        base.altered(|_, ds| ds.reverse()),
    );
    add(
        out,
        "control/decoy-digest-in-sd",
        "clause 4.2.5, decoy digests; clause 7.1 step 3.c.i ignores a digest nobody holds",
        base.altered(|p, _| sd(p).push(json!(digest_of("a decoy")))),
    );
    add(
        out,
        "control/decoy-placeholder-in-array",
        "clause 4.2.5; clause 7.1 step 3.d removes it",
        base.altered(|p, _| {
            p["nat"]
                .as_array_mut()
                .expect("an array")
                .push(json!({"...": digest_of("a decoy element")}));
        }),
    );
    let withheld = base.disclosure(&["nat", "#1"]);
    add(
        out,
        "control/withheld-array-element",
        "clause 7.1 step 3.d: a placeholder with no Disclosure is removed",
        base.altered(|_, ds| ds.retain(|d| *d != withheld)),
    );
    let kb = base.issuer.sign_typed(
        &json!({"iat": 1, "aud": "https://verifier.example", "nonce": "n", "sd_hash": "x"}),
        "kb+jwt",
    );
    add(
        out,
        "control/key-binding-segment-present",
        "clause 4.3; a verifier that is not asked for key binding does not need it",
        format!("{}{kb}", base.plain()),
    );
}

/// A payload whose structure breaks a rule of clause 7.1 or 4.
fn wrong_structure(base: &Base<'_>, out: &mut Vec<Adversarial>) {
    let given = base.disclosure(&["given_name"]);
    let given_digest = base.digest(&["given_name"]);
    let nat_digest = base.digest(&["nat", "#1"]);

    add(
        out,
        "reject/extra-disclosure-nothing-refers-to",
        "clause 7.1 step 5: every Disclosure must be used",
        base.altered(|_, ds| ds.push(raw(r#"["extra-salt","extra","x"]"#).0)),
    );
    add(
        out,
        "reject/disclosure-value-tampered",
        "clause 7.1: the digest no longer matches, so the original is unreferenced",
        base.altered(|_, ds| {
            let i = ds.iter().position(|d| *d == given).expect("present");
            ds[i] = raw(r#"["tampered-salt","given_name","Mallory"]"#).0;
        }),
    );
    add(
        out,
        "reject/disclosure-supplied-twice",
        "clause 7.1 step 4: a digest must not repeat",
        base.altered(|_, ds| ds.push(given.clone())),
    );
    add(
        out,
        "reject/digest-twice-in-one-sd-array",
        "clause 4.1: the same digest MUST NOT appear more than once",
        base.altered(|p, _| sd(p).push(json!(given_digest))),
    );
    add(
        out,
        "reject/digest-in-two-objects",
        "clause 4.1",
        base.altered(|p, _| {
            p["address"]["_sd"]
                .as_array_mut()
                .expect("address has _sd")
                .push(json!(given_digest));
        }),
    );
    add(
        out,
        "reject/digest-in-sd-and-in-an-array",
        "clause 4.1 and 7.1 step 3.c.iii.1",
        base.altered(|p, _| p["nat"][1] = json!({"...": given_digest})),
    );
    add(
        out,
        "reject/array-disclosure-named-by-an-sd-digest",
        "clause 7.1 step 3.c.ii.1: a two-element Disclosure where a three-element one belongs",
        base.altered(|p, _| {
            p["nat"][1] = json!("FR");
            sd(p).push(json!(nat_digest));
        }),
    );
    add(
        out,
        "reject/property-disclosure-named-by-a-placeholder",
        "clause 7.1 step 3.c.iii.1: a three-element Disclosure where a two-element one belongs",
        base.altered(|p, _| {
            sd(p).retain(|d| *d != json!(given_digest));
            p["nat"][1] = json!({"...": given_digest});
        }),
    );
    add(
        out,
        "reject/claim-collides-with-cleartext",
        "clause 7.1 step 3.c.ii.3",
        base.altered(|p, _| p["given_name"] = json!("Bob")),
    );
    add(
        out,
        "reject/two-disclosures-for-one-claim-name",
        "clause 7.1 step 3.c.ii.3",
        with_disclosure(base, r#"["other-salt","given_name","Other"]"#),
    );
    add(
        out,
        "reject/disclosure-names-the-claim-_sd",
        "clause 4.2.1 forbids the name; clause 7.1 step 3.c.ii.2",
        with_disclosure(base, r#"["s","_sd",["x"]]"#),
    );
    add(
        out,
        "reject/disclosure-names-the-claim-three-dots",
        "clause 4.2.1 forbids the name; clause 7.1 step 3.c.ii.2",
        with_disclosure(base, r#"["s","...","x"]"#),
    );
    add(
        out,
        "reject/unknown-hash-algorithm",
        "clause 7.1 step 2: the hash algorithm must be one the verifier supports",
        base.altered(|p, _| p["_sd_alg"] = json!("md5")),
    );
    add(
        out,
        "reject/sd-alg-nested-inside-an-object",
        "clause 4.1.1: _sd_alg is a claim of the payload's top level",
        base.altered(|p, _| p["address"]["_sd_alg"] = json!("sha-256")),
    );
    add(
        out,
        "reject/sd-is-not-an-array",
        "clause 4.1: _sd is an array",
        base.altered(|p, _| p["_sd"] = json!("abc")),
    );
    add(
        out,
        "reject/sd-holds-a-non-string",
        "clause 4.1: the array holds strings",
        base.altered(|p, _| sd(p).push(json!(1))),
    );
    add(
        out,
        "reject/placeholder-with-a-second-key",
        "clause 4.2.2: the placeholder is an object with the one key",
        base.altered(|p, _| p["nat"][1] = json!({"...": nat_digest, "x": 1})),
    );
    add(
        out,
        "reject/placeholder-value-not-a-string",
        "clause 4.2.2: the digest is a string",
        base.altered(|p, _| p["nat"][1] = json!({"...": 5})),
    );
    add(
        out,
        "reject/jwt-payload-is-an-array",
        "clause 3: the payload is a JSON object",
        base.token(&json!([1, 2]), &[]),
    );
}

/// Disclosures that are not the arrays clause 4.2 defines.
fn malformed_disclosures(base: &Base<'_>, out: &mut Vec<Adversarial>) {
    for (id, text, clause) in [
        (
            "one-element",
            r#"["salt"]"#,
            "clause 4.2: two or three elements",
        ),
        (
            "four-elements",
            r#"["salt","name","value","more"]"#,
            "clause 4.2: two or three elements",
        ),
        ("an-object", r#"{"a":1}"#, "clause 4.2: a JSON array"),
        ("a-string", r#""just a string""#, "clause 4.2: a JSON array"),
        (
            "salt-is-a-number",
            r#"[1,"name","value"]"#,
            "clause 4.2.1: the salt is a string",
        ),
        (
            "name-is-a-number",
            r#"["salt",5,"value"]"#,
            "clause 4.2.1: the claim name is a string",
        ),
        ("not-json", "not json at all", "clause 4.2: JSON"),
        (
            "json-then-garbage",
            r#"["salt","name","value"] trailing"#,
            "clause 4.2: the whole content is JSON",
        ),
    ] {
        add(
            out,
            &format!("reject/disclosure-{id}"),
            clause,
            with_disclosure(base, text),
        );
    }

    add(
        out,
        "reject/disclosure-is-not-base64url",
        "clause 4.2: base64url",
        base.altered(|p, ds| {
            let bad = "!!not*base64url!!".to_owned();
            sd(p).push(json!(digest_of(&bad)));
            ds.push(bad);
        }),
    );

    // The digest is of the padded string, so padding is the only defect.
    let (encoded, _) = raw(r#"["s","n","a"]"#);
    assert_eq!(
        encoded.len() % 4,
        2,
        "this text encodes with padding to add"
    );
    let padded = format!("{encoded}==");
    add(
        out,
        "reject/disclosure-base64url-with-padding",
        "clause 4.2 and RFC 7515 section 2: base64url without padding",
        base.altered(|p, ds| {
            sd(p).push(json!(digest_of(&padded)));
            ds.push(padded.clone());
        }),
    );
}

/// The compact serialisation itself.
fn malformed_serialisation(base: &Base<'_>, out: &mut Vec<Adversarial>) {
    let plain = base.plain();
    add(
        out,
        "reject/last-disclosure-missing-its-closing-tilde",
        "clause 4: the serialisation ends with ~ when there is no Key Binding JWT",
        plain.trim_end_matches('~').to_owned(),
    );
    add(
        out,
        "reject/no-tilde-at-all",
        "clause 4: JWT~D1~…~DN~",
        base.issuer.sign(&base.payload()),
    );
    add(
        out,
        "reject/jwt-has-two-parts",
        "RFC 7515: three parts",
        format!(
            "{}~",
            base.issuer
                .sign(&base.payload())
                .rsplit_once('.')
                .expect("a JWS")
                .0
        ),
    );
    add(out, "reject/empty-string", "clause 4", String::new());
}

/// Valid or not depending on a choice the RFC leaves open, or on a limit.
fn edges(base: &Base<'_>, out: &mut Vec<Adversarial>) {
    add(
        out,
        "edge/disclosure-json-with-whitespace",
        "clause 4.2: any JSON; this crate's own encoder writes a compact form",
        with_disclosure(base, "[ \"salt\" ,\n\t\"spaced\" , \"value\" ]"),
    );
    add(
        out,
        "edge/empty-claim-name",
        "clause 4.2.1 does not forbid the empty name",
        with_disclosure(base, r#"["s","",1]"#),
    );
    add(
        out,
        "edge/claim-name-spelled-with-unicode-escapes",
        "RFC 8259: an escape and the character it spells are the same string",
        with_disclosure(base, r#"["s","café","v"]"#),
    );
    add(
        out,
        "edge/duplicate-key-in-a-disclosed-object",
        "RFC 8259 leaves a repeated name's meaning to the receiver",
        with_disclosure(base, r#"["s","obj",{"a":1,"a":2}]"#),
    );

    // SHA-512 is a hash algorithm the RFC allows and this crate does not
    // implement. Built here with real SHA-512 digests, so the one thing a
    // verifier can object to is this crate's choice of algorithm.
    let (encoded, _) = raw(r#"["salt","given_name","Alice"]"#);
    let digest512 = B64.encode(Sha512::digest(encoded.as_bytes()));
    add(
        out,
        "edge/hash-algorithm-sha-512",
        "clause 4.1.1 allows any IANA hash; this crate implements SHA-256 only, on purpose",
        base.token(
            &json!({"iss": "https://issuer.example", "_sd": [digest512], "_sd_alg": "sha-512"}),
            &[encoded],
        ),
    );

    for depth in [40, 200] {
        let mut value = json!("leaf");
        for _ in 0..depth {
            value = json!({"d": value});
        }
        let text = format!(r#"["s","deep",{value}]"#);
        add(
            out,
            &format!("edge/nested-{depth}-deep"),
            "no clause: a limit on nesting is the receiver's to set",
            with_disclosure(base, &text),
        );
    }

    let wide = (0..1000).fold(Map::new(), |mut m, n| {
        m.insert(format!("c{n:04}"), json!(n));
        m
    });
    let hidden: std::collections::BTreeSet<Path> =
        wide.keys().map(|k| path(&[k.as_str()])).collect();
    let issued = issue_to_plan(&Value::Object(wide), &hidden);
    let disclosures: Vec<String> = issued
        .disclosures
        .iter()
        .map(|d| d.encoded().to_owned())
        .collect();
    add(
        out,
        "edge/a-thousand-disclosures",
        "no clause: a limit on the number of Disclosures is the receiver's to set",
        assemble(&base.issuer.sign(&issued.payload), &disclosures),
    );
}
