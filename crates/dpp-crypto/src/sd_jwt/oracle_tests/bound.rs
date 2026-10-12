//! Presentations bound to a Holder's key, judged with Key Binding required.
//!
//! RFC 9901 clauses 4.1.2, 4.3 and 7.3. The Issuer names the Holder's Ed25519 key
//! in `cnf`, this crate's Holder binds the presentation, and every case carries
//! the challenge a Verifier would hold it to. The controls are bound by
//! [`SdJwt::present_with_key_binding`] itself. Every other case is one of them with
//! one thing changed and the Key Binding JWT signed again, so that the change is
//! the only fault, and its id says whether a reader must accept or refuse it.

use serde_json::{Map, Value, json};

use crate::jws::signer::sign_with_header;
use crate::keystore::KeyEntry;
use crate::sd_jwt::{KeyBinding, KeyBindingPolicy, SdJwt, digest_of, holder_key_confirmation};

use super::plan::{issue_to_plan, path, paths};
use super::signing::{Issuer, holder};

/// The Verifier every case is for.
pub(super) const AUDIENCE: &str = "https://verifier.example";
/// The challenge every case answers.
pub(super) const NONCE: &str = "n-2RsH_0kQ8vLz";
/// How far `iat` may lie from now either way, in seconds. One number for both
/// directions, because one of the other implementations takes its window as a
/// point and a symmetric skew.
pub(super) const WINDOW: i64 = 300;
/// Far enough outside the window that no clock a judge reads can bring it back.
const FAR: i64 = 3600;

/// RFC 9901 Appendix A.5's key, a P-256 key that is a point on its curve, as a
/// Holder key this crate does not verify with.
const P256_JWK: &str = r#"{"kty":"EC","crv":"P-256","x":"b28d4MwZMjw8-00CG4xfnn9SLMVMM19SlqZpVb_uNtQ","y":"Xv5zWwuoaTgdS6hV43yI6gBwTnjukmFQQnJ_kCxzqk8"}"#;

pub(super) struct Bound {
    pub(super) id: String,
    /// The clause of RFC 9901 the case turns on.
    pub(super) clause: &'static str,
    pub(super) token: String,
}

/// The policy a Verifier holds every case to, at `now`.
pub(super) fn policy(now: i64) -> KeyBindingPolicy {
    KeyBindingPolicy {
        audience: AUDIENCE.to_owned(),
        nonce: NONCE.to_owned(),
        now,
        max_age: WINDOW,
        leeway: WINDOW,
    }
}

/// A credential issued to the Holder whose `cnf` is given, hiding a name, a
/// member of an object and an array element.
fn issued(issuer: &Issuer, cnf: Option<Value>, cnf_hidden: bool) -> SdJwt {
    let mut claims = json!({
        "iss": "https://issuer.example",
        "given_name": "Erika",
        "family_name": "Mustermann",
        "address": { "locality": "Berlin", "country": "DE" },
        "nationalities": ["DE", "FR"],
    });
    if let Some(cnf) = cnf {
        claims["cnf"] = cnf;
    }
    let mut hidden = paths(&[
        &["given_name"],
        &["family_name"],
        &["address", "locality"],
        &["nationalities", "#1"],
    ]);
    if cnf_hidden {
        hidden.insert(path(&["cnf"]));
    }
    let plan = issue_to_plan(&claims, &hidden);
    SdJwt::new(issuer.sign(&plan.payload), plan.disclosures)
}

fn every(sd_jwt: &SdJwt) -> Vec<String> {
    sd_jwt.disclosures().iter().map(|d| d.digest()).collect()
}

fn refs(digests: &[String]) -> Vec<&str> {
    digests.iter().map(String::as_str).collect()
}

fn binding(now: i64) -> KeyBinding {
    KeyBinding {
        audience: AUDIENCE.to_owned(),
        nonce: NONCE.to_owned(),
        issued_at: now,
    }
}

/// `<presentation>~<KB-JWT>`, the Key Binding JWT signed by `key` over `header`
/// and `claims`.
fn bind(presentation: &SdJwt, key: &KeyEntry, header: &Value, claims: &Value) -> String {
    let header: Map<String, Value> = header.as_object().expect("an object").clone();
    let kb = sign_with_header(key, &header, claims.to_string().as_bytes());
    format!("{}{kb}", presentation.serialise())
}

fn add(out: &mut Vec<Bound>, id: &str, clause: &'static str, token: String) {
    out.push(Bound {
        id: id.to_owned(),
        clause,
        token,
    });
}

pub(super) fn cases(issuer: &Issuer, now: i64) -> Vec<Bound> {
    let key = holder("sdjwt-oracle-holder");
    let stranger = holder("sdjwt-oracle-stranger");
    let cnf = holder_key_confirmation(&key.verifying_key.to_bytes());
    let sd_jwt = issued(issuer, Some(cnf.clone()), false);
    let all = every(&sd_jwt);
    let mut out = Vec::new();

    let bound = |reveal: &[&str]| {
        sd_jwt
            .present_with_key_binding(reveal, &binding(now), &key)
            .expect("the Holder binds")
            .serialise()
    };
    add(
        &mut out,
        "control/every-disclosure",
        "clause 7.3",
        bound(&refs(&all)),
    );
    add(
        &mut out,
        "control/no-disclosure",
        "clause 4.3.1: the hash of <JWT>~",
        bound(&[]),
    );
    let mut declared = cnf.clone();
    declared["jwk"]["alg"] = json!("EdDSA");
    add(
        &mut out,
        "control/holder-key-declares-its-algorithm",
        "RFC 7517: alg names the algorithm the key is for",
        issued(issuer, Some(declared), false)
            .present_with_key_binding(&[], &binding(now), &key)
            .expect("the Holder binds")
            .serialise(),
    );

    // The presentation that every altered case starts from, and its good claims.
    let base = sd_jwt.present(&refs(&all)).expect("presents");
    let header = json!({ "alg": "EdDSA", "typ": "kb+jwt" });
    let claims = json!({ "iat": now, "aud": AUDIENCE, "nonce": NONCE, "sd_hash": base.sd_hash() });
    let with = |field: &str, value: Value| {
        let mut c = claims.clone();
        c[field] = value;
        c
    };
    let without = |field: &str| {
        let mut c = claims.clone();
        c.as_object_mut().expect("an object").remove(field);
        c
    };
    let header_with = |field: &str, value: Value| {
        let mut h = header.clone();
        h[field] = value;
        h
    };

    add(
        &mut out,
        "reject/key-binding-jwt-missing",
        "clause 7.3 step 2",
        base.serialise(),
    );
    let one = sd_jwt.present(&refs(&all[..1])).expect("presents");
    let lifted = bound(&refs(&all[..1]));
    let (_, kb) = lifted.rsplit_once('~').expect("bound");
    add(
        &mut out,
        "reject/key-binding-jwt-moved-to-another-selection",
        "clause 4.3.1: sd_hash covers the Disclosures presented",
        format!("{}{kb}", base.serialise()),
    );
    let unterminated = base.serialise();
    let unterminated = &unterminated[..unterminated.len() - 1];
    add(
        &mut out,
        "reject/sd-hash-without-the-closing-tilde",
        "clause 4.3.1: the hash is over <JWT>~<D.1>~...~<D.N>~",
        bind(
            &base,
            &key,
            &header,
            &with("sd_hash", json!(digest_of(unterminated))),
        ),
    );
    add(
        &mut out,
        "reject/sd-hash-of-the-issued-token",
        "clause 4.3.1: the hash is over what is presented, not what was issued",
        bind(
            &one,
            &key,
            &header,
            &with("sd_hash", json!(sd_jwt.sd_hash())),
        ),
    );
    for (id, clause, signer, h, c) in [
        (
            "reject/signed-by-another-key",
            "clause 7.3 step 5.c",
            &stranger,
            header.clone(),
            claims.clone(),
        ),
        (
            "reject/signed-by-the-issuer",
            "clause 7.3 step 5.a: the Holder's key, from cnf",
            &issuer.key(),
            header.clone(),
            claims.clone(),
        ),
        (
            "reject/alg-none",
            "clause 4.3 and 7.3 step 5.b: alg MUST NOT be none",
            &key,
            header_with("alg", json!("none")),
            claims.clone(),
        ),
        (
            "reject/crit-names-an-extension",
            "RFC 7515 clause 4.1.11",
            &key,
            json!({ "alg": "EdDSA", "typ": "kb+jwt", "crit": ["urn:example:x"], "urn:example:x": 1 }),
            claims.clone(),
        ),
        (
            "reject/typ-is-jwt",
            "clause 4.3 and 7.3 step 5.d",
            &key,
            header_with("typ", json!("JWT")),
            claims.clone(),
        ),
        (
            "reject/typ-missing",
            "clause 4.3: typ is REQUIRED",
            &key,
            json!({ "alg": "EdDSA" }),
            claims.clone(),
        ),
        (
            "reject/iat-too-old",
            "clause 7.3 step 5.e",
            &key,
            header.clone(),
            with("iat", json!(now - FAR)),
        ),
        (
            "reject/iat-in-the-future",
            "clause 7.3 step 5.e",
            &key,
            header.clone(),
            with("iat", json!(now + FAR)),
        ),
        (
            "reject/iat-missing",
            "clause 4.3: iat is REQUIRED",
            &key,
            header.clone(),
            without("iat"),
        ),
        (
            "reject/iat-is-a-string",
            "clause 4.3: iat is a NumericDate (RFC 7519)",
            &key,
            header.clone(),
            with("iat", json!(now.to_string())),
        ),
        (
            "reject/nonce-is-another",
            "clause 7.3 step 5.f",
            &key,
            header.clone(),
            with("nonce", json!("n-replayed")),
        ),
        (
            "reject/nonce-missing",
            "clause 4.3: nonce is REQUIRED",
            &key,
            header.clone(),
            without("nonce"),
        ),
        (
            "reject/aud-is-another-verifier",
            "clause 7.3 step 5.f",
            &key,
            header.clone(),
            with("aud", json!("https://elsewhere.example")),
        ),
        (
            "reject/aud-missing",
            "clause 4.3: aud is REQUIRED",
            &key,
            header.clone(),
            without("aud"),
        ),
        (
            "reject/aud-is-an-array",
            "clause 4.3: aud MUST be a single string",
            &key,
            header.clone(),
            with("aud", json!([AUDIENCE])),
        ),
        (
            "reject/sd-hash-is-another",
            "clause 7.3 step 5.g",
            &key,
            header.clone(),
            with("sd_hash", json!(digest_of("another"))),
        ),
        (
            "reject/sd-hash-missing",
            "clause 4.3: sd_hash is REQUIRED",
            &key,
            header.clone(),
            without("sd_hash"),
        ),
        (
            "reject/expired",
            "clause 7.3 step 5.h: exp (RFC 7519)",
            &key,
            header.clone(),
            with("exp", json!(now - FAR)),
        ),
        (
            "reject/not-yet-valid",
            "clause 7.3 step 5.h: nbf (RFC 7519)",
            &key,
            header.clone(),
            with("nbf", json!(now + FAR)),
        ),
        (
            "edge/typ-with-the-application-prefix",
            "RFC 7515 clause 4.1.9: kb+jwt and application/kb+jwt are one media type",
            &key,
            header_with("typ", json!("application/kb+jwt")),
            claims.clone(),
        ),
    ] {
        add(&mut out, id, clause, bind(&base, signer, &h, &c));
    }

    // The Issuer's side of the binding: no key named, a key of another kind, and
    // a key that declares an algorithm it is not.
    let unnamed = issued(issuer, None, false);
    let p256: Value = serde_json::from_str(P256_JWK).expect("JSON");
    let mut misdeclared = cnf.clone();
    misdeclared["jwk"]["alg"] = json!("ES256");
    for (id, clause, cnf) in [
        (
            "reject/no-holder-key",
            "clause 4.1.2 and 7.3 step 5.a",
            None,
        ),
        (
            "reject/holder-key-is-p-256",
            "this crate binds with Ed25519 keys only",
            Some(json!({ "jwk": p256 })),
        ),
        (
            "reject/holder-key-declares-another-algorithm",
            "RFC 7517 section 4.4 and RFC 8725 section 3.1: one key, one algorithm",
            Some(misdeclared),
        ),
    ] {
        let sd_jwt = match &cnf {
            None => unnamed.clone(),
            Some(cnf) => issued(issuer, Some(cnf.clone()), false),
        };
        let presented = sd_jwt.present(&[]).expect("presents");
        let c =
            json!({ "iat": now, "aud": AUDIENCE, "nonce": NONCE, "sd_hash": presented.sd_hash() });
        add(&mut out, id, clause, bind(&presented, &key, &header, &c));
    }

    // `cnf` itself selectively disclosed, and disclosed.
    let concealed = issued(issuer, Some(cnf), true);
    let cnf_digest: Vec<String> = concealed
        .disclosures()
        .iter()
        .filter(|d| d.claim_name() == Some("cnf"))
        .map(|d| d.digest())
        .collect();
    add(
        &mut out,
        "edge/holder-key-disclosed-not-in-the-clear",
        "clause 7.3 step 5.a: the Holder's key, from the SD-JWT",
        concealed
            .present_with_key_binding(&refs(&cnf_digest), &binding(now), &key)
            .expect("the Holder binds")
            .serialise(),
    );
    out
}
