//! A real SD-JWT VC for every product group, for an independent SD-JWT library to
//! read, and the credentials that library issues, for `dpp_vc` to verify.
//!
//! # Why this exists
//!
//! `dpp-crypto`'s oracle corpus judges the SD-JWT mechanism on claim trees built
//! for the purpose. A passport is not one of those. This issues the credential
//! `dpp_vc::sd_jwt_vc::issue` would issue for each product group in the catalog,
//! from that group's schema-compat fixture and the disclosure classes of its
//! current schema version, so the concealment the access policy asks for is what
//! the other implementation reads.
//!
//! Judged at **RFC 9901 level only**. The credential profile is an Internet-Draft,
//! and this workspace claims no conformance to it, so nothing here is evidence of
//! any. The one profile check made is the `kid`, and that is RFC 7638 and RFC 9278.
//!
//! # What the other implementation checks
//!
//! `.github/oracle/sdjwt/credentials.mjs` reads the corpus. For the credential as
//! issued, it must arrive at the passport that went in, plus the three claims the
//! issuer adds (`iss`, `vct`, `iat`). For the presentations built from it, it must
//! agree with this crate. The signature is checked there with Node's `crypto`, and
//! the `kid` is recomputed from the public key.
//!
//! # The other direction
//!
//! `CHECK_SDJWT_VC_ORACLE_ISSUED` names a file of credentials the other library
//! issued, with a `typ` of `dc+sd-jwt`, a `vct`, a validity window and what the
//! library makes of each. `dpp_vc::sd_jwt_vc::verify` has to reach the same
//! verdict and the same claims.
//!
//! # Running
//!
//! ```text
//! EMIT_SDJWT_VC_CORPUS=1 cargo test -p dpp-tests --test sdjwt_vc_oracle_corpus
//! CHECK_SDJWT_VC_ORACLE_ISSUED=target/sdjwt-oracle/issued-vc.json \
//!   cargo test -p dpp-tests --test sdjwt_vc_oracle_corpus
//! ```
//!
//! Without the variables the first test still runs and still checks that the
//! corpus is complete and reads back, so an ordinary `just check` gets that
//! coverage without producing files.

use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use chrono::{DateTime, TimeZone, Utc};
use dpp_crypto::keystore::KeyStore;
use dpp_crypto::sd_jwt::SdJwt;
use dpp_domain::access::ProductGroupAccessPolicy;
use dpp_domain::{ProductGroup, ProductGroupCatalog, ProductGroupData};
use dpp_tests::fixtures::base_passport;
use dpp_vc::sd_jwt_vc::{issue, vct_for, verify};
use serde_json::{Map, Number, Value, json};

const ISSUER: &str = "https://passports.operator.example";
const KEY_ID: &str = "issuer";
/// A fixed instant, so nothing here depends on the wall clock.
const ISSUED_AT: i64 = 1_789_000_000;
/// How many Disclosures are presented one at a time per credential.
const ONE_AT_A_TIME: usize = 12;

fn now() -> DateTime<Utc> {
    Utc.timestamp_opt(ISSUED_AT + 60, 0).unwrap()
}

/// The group's fixture at its current schema version.
fn fixture_for(key: &str, version: &str) -> Value {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../dpp-domain/tests/fixtures/schema-compat")
        .join(key)
        .join(format!("v{version}.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("no schema-compat fixture for '{key}' at {version}: {e}"));
    serde_json::from_str(&raw).expect("a fixture is JSON")
}

/// A passport's wire form with its product group data replaced by `data`.
///
/// The envelope comes from the real serialiser, and only `productGroupData` is
/// swapped, as JSON, so the credential carries exactly the keys the fixture holds.
fn passport_with(group: &ProductGroup, version: &str, data: Value) -> Value {
    let placeholder: ProductGroupData = serde_json::from_value(json!({
        "productGroup": "packaging",
        "materialFamily": "placeholder",
    }))
    .expect("an unmodelled product group deserialises");
    let mut passport = serde_json::to_value(base_passport(group.clone(), placeholder, version))
        .expect("a passport serialises");

    let tag = serde_json::to_value(group).expect("a product group serialises");
    let mut data = data;
    data.as_object_mut()
        .expect("product group data is an object")
        .insert("productGroup".to_owned(), tag.clone());
    let wire = passport.as_object_mut().expect("a passport is an object");
    wire.insert("productGroup".to_owned(), tag);
    wire.insert("productGroupData".to_owned(), data);
    passport
}

struct Issuer {
    store: KeyStore,
    path: PathBuf,
    public_key_b64: String,
    kid: String,
}

impl Issuer {
    fn new() -> Self {
        // Cargo's scratch directory for integration tests, inside `target`, and
        // not the system temporary directory, which other users share.
        let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "test-sdjwt-vc-corpus-{}.json",
            uuid::Uuid::now_v7()
        ));
        // A new passphrase per run, for a store that lives for one test.
        let passphrase = uuid::Uuid::now_v7().to_string();
        let store = KeyStore::open(&path, &passphrase).expect("open the keystore");
        store.generate_key(KEY_ID).expect("generate the key");
        let info = store
            .public_key(KEY_ID)
            .expect("the key was just generated");
        let raw = hex::decode(&info.verifying_key_hex).expect("a hex key");
        Self {
            public_key_b64: B64.encode(raw),
            kid: info.thumbprint_uri().expect("a thumbprint"),
            store,
            path,
        }
    }

    fn jwk(&self) -> Value {
        json!({ "kty": "OKP", "crv": "Ed25519", "x": self.public_key_b64 })
    }
}

impl Drop for Issuer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// What `dpp_vc` makes of a token: accepted with these claims, or refused.
fn verdict(token: &str, issuer: &Issuer, vct: &str) -> Value {
    match verify(token, &issuer.public_key_b64, vct, now()) {
        Ok(claims) => json!({ "accepts": true, "payload": claims }),
        Err(e) => json!({ "accepts": false, "error": format!("{e:?}") }),
    }
}

/// The claims a credential issued from `passport` must reveal in full: the
/// passport, and the three claims the issuer sets.
fn full_disclosure_of(passport: &Value, vct: &str) -> Value {
    let mut claims: Map<String, Value> = passport.as_object().expect("an object").clone();
    for reserved in ["iss", "vct", "iat"] {
        assert!(
            !claims.contains_key(reserved),
            "the passport envelope already uses '{reserved}', which the issuer sets"
        );
    }
    claims.insert("iss".to_owned(), json!(ISSUER));
    claims.insert("vct".to_owned(), json!(vct));
    claims.insert("iat".to_owned(), json!(ISSUED_AT));
    Value::Object(claims)
}

fn corpus(issuer: &Issuer) -> Vec<Value> {
    let mut cases = Vec::new();
    for descriptor in ProductGroupCatalog::new().all().iter() {
        let key = descriptor.key.as_str();
        let version = descriptor.current_schema_version.as_str();
        let group = ProductGroup::from_wire_tag(key);
        let passport = passport_with(&group, version, fixture_for(key, version));
        let policy = ProductGroupAccessPolicy::for_passport(key, version)
            .unwrap_or_else(|| panic!("no access policy for {key} {version}"));
        let vct = vct_for(group, version);

        let credential = issue(
            &issuer.store,
            KEY_ID,
            &passport,
            &policy,
            ISSUER,
            &vct,
            ISSUED_AT,
        )
        .unwrap_or_else(|e| panic!("issuing a {key} credential: {e:?}"));
        let digests: Vec<String> = credential
            .disclosures()
            .iter()
            .map(dpp_crypto::sd_jwt::Disclosure::digest)
            .collect();

        let mut presentations: Vec<(String, Vec<&str>)> = vec![
            (
                "issued".to_owned(),
                digests.iter().map(String::as_str).collect(),
            ),
            ("withheld".to_owned(), Vec::new()),
        ];
        for (n, digest) in digests.iter().take(ONE_AT_A_TIME).enumerate() {
            presentations.push((format!("only-{n}"), vec![digest.as_str()]));
        }

        for (name, reveal) in presentations {
            let token = credential.present(&reveal).serialise();
            let id = format!("{key}/{version}/{name}");
            let mut case = json!({
                "id": id,
                "productGroup": key,
                "schemaVersion": version,
                "disclosures": reveal.len(),
                "token": token,
                "ours": verdict(&token, issuer, &vct),
            });
            if name == "issued" {
                case["expected"] = full_disclosure_of(&passport, &vct);
            }
            cases.push(case);
        }
    }
    cases
}

#[test]
fn every_product_group_has_a_credential_that_reads_back_to_its_passport() {
    let issuer = Issuer::new();
    let cases = corpus(&issuer);

    let mut wrong = Vec::new();
    for case in &cases {
        let id = case["id"].as_str().unwrap_or("?");
        if case["ours"]["accepts"] != json!(true) {
            wrong.push(format!("{id}: refused: {}", case["ours"]["error"]));
        } else if let Some(expected) = case.get("expected")
            && let Some(at) = first_difference(&case["ours"]["payload"], expected, "")
        {
            wrong.push(format!(
                "{id}: the credential does not reveal the passport it was issued from, at {at}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));

    let catalog = ProductGroupCatalog::new();
    for descriptor in catalog.all().iter() {
        let id = format!(
            "{}/{}/issued",
            descriptor.key, descriptor.current_schema_version
        );
        assert!(
            cases.iter().any(|c| c["id"] == json!(id)),
            "the corpus holds no credential for product group '{}'",
            descriptor.key
        );
    }

    // A credential that conceals nothing proves nothing about concealment, and the
    // classes are the point. Withholding every Disclosure must change what is
    // revealed, for most groups: a group with nothing to hide is the exception.
    let payload = |id: &str| {
        cases
            .iter()
            .find(|c| c["id"] == json!(id))
            .map(|c| c["ours"]["payload"].clone())
    };
    let concealing = catalog
        .all()
        .iter()
        .filter(|d| {
            let stem = format!("{}/{}", d.key, d.current_schema_version);
            payload(&format!("{stem}/issued")) != payload(&format!("{stem}/withheld"))
        })
        .count();
    assert!(
        concealing >= catalog.all().len() / 2,
        "only {concealing} of {} credentials conceal anything",
        catalog.all().len()
    );

    if std::env::var_os("EMIT_SDJWT_VC_CORPUS").is_none() {
        return;
    }
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/sdjwt-oracle");
    std::fs::create_dir_all(&dir).expect("create the oracle output directory");
    let body = json!({
        "issuerJwk": issuer.jwk(),
        "kid": issuer.kid,
        "currentDate": ISSUED_AT + 60,
        "cases": cases,
    });
    std::fs::write(
        dir.join("credentials.json"),
        serde_json::to_string(&body).expect("serialises"),
    )
    .expect("write the corpus");
    eprintln!("wrote {} credentials to {}", cases.len(), dir.display());
}

/// Every way `file` disagrees with `dpp_vc::sd_jwt_vc::verify`.
///
/// `file` is `{ "publicKey": <base64url raw Ed25519 key>, "now": <seconds>,
/// "cases": [...] }`, and a case is `{ "id", "token", "vct", "verdict": "accept" |
/// "refuse", "expected"? }`.
fn disagreements(file: &Value) -> Vec<String> {
    let key = file["publicKey"].as_str().expect("the file names its key");
    let now = Utc
        .timestamp_opt(file["now"].as_i64().expect("the file names a time"), 0)
        .unwrap();
    let mut out = Vec::new();
    for case in file["cases"].as_array().expect("the file holds cases") {
        let id = case["id"].as_str().unwrap_or("?");
        let token = case["token"].as_str().expect("a case has a token");
        let vct = case["vct"]
            .as_str()
            .expect("a case names the vct it expects");
        match (case["verdict"].as_str(), verify(token, key, vct, now)) {
            (Some("accept"), Ok(claims)) => {
                // `expected` is optional in the file: a case without it is judged
                // on its verdict alone.
                if let Some(expected) = case.get("expected")
                    && let Some(at) = first_difference(&Value::Object(claims), expected, "")
                {
                    out.push(format!(
                        "{id}: reads differently from the other implementation, at {at}"
                    ));
                }
            }
            (Some("accept"), Err(why)) => {
                out.push(format!(
                    "{id}: refused a credential the other issued: {why:?}"
                ));
            }
            (Some("refuse"), Ok(_)) => {
                out.push(format!("{id}: accepted a credential that must be refused"));
            }
            (Some("refuse"), Err(_)) => {}
            (other, _) => out.push(format!("{id}: unknown verdict {other:?}")),
        }
    }
    out
}

#[test]
fn what_an_independent_implementation_issued_verifies_here() {
    let Some(given) = std::env::var_os("CHECK_SDJWT_VC_ORACLE_ISSUED") else {
        return;
    };
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(given);
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let file: Value = serde_json::from_str(&text).expect("the file is JSON");

    let count = file["cases"].as_array().map_or(0, Vec::len);
    assert!(count >= 10, "only {count} credentials to verify");
    let found = disagreements(&file);
    assert!(
        found.is_empty(),
        "{} of {count} credentials issued by an independent implementation do not verify \
         the way it says:\n{}",
        found.len(),
        found.join("\n")
    );
    eprintln!("verified {count} credentials issued by an independent implementation");
}

#[test]
fn the_verify_check_fails_on_a_mismatch() {
    let issuer = Issuer::new();
    let group = ProductGroup::from_wire_tag("battery");
    let version = "2.6.0";
    let passport = passport_with(&group, version, fixture_for("battery", version));
    let policy = ProductGroupAccessPolicy::for_passport("battery", version).expect("a policy");
    let vct = vct_for(group, version);
    let credential = issue(
        &issuer.store,
        KEY_ID,
        &passport,
        &policy,
        ISSUER,
        &vct,
        ISSUED_AT,
    )
    .expect("issues");
    let token: String = credential.serialise();
    let expected = full_disclosure_of(&passport, &vct);

    let file = |verdict: &str, vct: &str, expected: &Value| {
        json!({
            "publicKey": issuer.public_key_b64,
            "now": ISSUED_AT + 60,
            "cases": [{ "id": "case", "verdict": verdict, "token": token, "vct": vct, "expected": expected }]
        })
    };
    assert!(disagreements(&file("accept", &vct, &expected)).is_empty());
    assert_eq!(
        disagreements(&file("accept", &vct, &json!({}))).len(),
        1,
        "a wrong expectation must be reported"
    );
    assert_eq!(
        disagreements(&file("refuse", &vct, &expected)).len(),
        1,
        "a credential claimed unreadable that reads fine must be reported"
    );
    assert_eq!(
        disagreements(&file("accept", "tag:elsewhere:vct", &expected)).len(),
        1,
        "a credential for another type must be reported"
    );
    // The parse is the first step; a sanity check that the token is one.
    assert!(SdJwt::parse(&token).is_ok());
}

/// Past 2^53 a double cannot tell two integers apart, so the comparison that
/// forgives `12` against `12.0` must not forgive those.
#[test]
fn numbers_are_compared_exactly_where_a_double_cannot_hold_them() {
    let n = |text: &str| match serde_json::from_str::<Value>(text).expect("JSON") {
        Value::Number(number) => number,
        other => panic!("{text} is not a number: {other}"),
    };
    for (a, b) in [
        ("12", "12.0"),
        ("0", "-0.0"),
        ("9007199254740992", "9007199254740992.0"),
    ] {
        assert!(
            same_number(&n(a), &n(b)) && same_number(&n(b), &n(a)),
            "{a} and {b}"
        );
    }
    for (a, b) in [
        ("12", "12.5"),
        ("9007199254740993", "9007199254740992"),
        ("9007199254740993", "9007199254740992.0"),
        ("18446744073709551615", "18446744073709551616.0"),
    ] {
        assert!(
            !same_number(&n(a), &n(b)) && !same_number(&n(b), &n(a)),
            "{a} and {b}"
        );
    }
}

/// Whether two JSON numbers are one number.
///
/// `12.0` and `12` are: the issuer canonicalises numbers (RFC 8785), so a
/// whole-number float in a passport comes back whole. Two integers are compared
/// exactly and never through a double, because past 2^53 distinct integers share
/// one. An integer and a float agree only when the float is whole and no larger
/// than 2^53, where a double holds every integer exactly.
fn same_number(x: &Number, y: &Number) -> bool {
    /// 2^53.
    const EXACT_LIMIT: f64 = 9_007_199_254_740_992.0;
    let whole_float_is = |float: &Number, integer: &Number| {
        let Some(f) = float
            .as_f64()
            .filter(|f| f.fract() == 0.0 && f.abs() <= EXACT_LIMIT)
        else {
            return false;
        };
        // Digits, not a cast: `-0.0` is `0`, and everything else prints whole.
        let digits = if f == 0.0 {
            "0".to_owned()
        } else {
            format!("{f:.0}")
        };
        digits == integer.to_string()
    };
    x == y
        || (x.is_f64() && !y.is_f64() && whole_float_is(x, y))
        || (y.is_f64() && !x.is_f64() && whole_float_is(y, x))
}

/// Where two documents first differ, as a path, so a failure names the claim.
fn first_difference(a: &Value, b: &Value, path: &str) -> Option<String> {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for key in x.keys().chain(y.keys()) {
                let here = format!("{path}/{key}");
                match (x.get(key), y.get(key)) {
                    (Some(l), Some(r)) => {
                        if let Some(found) = first_difference(l, r, &here) {
                            return Some(found);
                        }
                    }
                    (Some(_), None) => return Some(format!("{here}: only in the first")),
                    (None, Some(_)) => return Some(format!("{here}: only in the second")),
                    (None, None) => {}
                }
            }
            None
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => x
            .iter()
            .zip(y)
            .enumerate()
            .find_map(|(i, (l, r))| first_difference(l, r, &format!("{path}/{i}"))),
        (Value::Number(x), Value::Number(y)) if same_number(x, y) => None,
        _ if a == b => None,
        _ => Some(format!("{path}: {a} against {b}")),
    }
}
