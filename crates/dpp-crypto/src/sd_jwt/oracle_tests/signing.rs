//! The issuer key the corpus is signed with, how a token is assembled, and the
//! verdict this crate gives one.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::{Value, json};

use crate::jws::sign_typed;
use crate::jws::verifier::verify_jws;
use crate::keystore::KeyStore;
use crate::sd_jwt::SdJwt;
use crate::test_support::temp_store;

const KEY_ID: &str = "sdjwt-oracle-issuer";

/// The `typ` the RFC's own examples carry. The RFC does not require one; an
/// issuer that sets it names the kind of token explicitly (clause 9.11).
pub(super) const TYP: &str = "example+sd-jwt";

/// An Ed25519 issuer, the algorithm this crate signs with.
pub(super) struct Issuer {
    store: KeyStore,
    kid: String,
    /// The raw public key, base64url, the form [`verify_jws`] takes.
    pub(super) public_key_b64: String,
}

impl Issuer {
    pub(super) fn new() -> Self {
        let store = temp_store("sdjwt-oracle", KEY_ID);
        let info = store
            .public_key(KEY_ID)
            .expect("the key was just generated");
        let kid = info.thumbprint_uri().expect("a thumbprint");
        let raw = hex::decode(&info.verifying_key_hex).expect("a hex key");
        Self {
            store,
            kid,
            public_key_b64: URL_SAFE_NO_PAD.encode(raw),
        }
    }

    /// The public key as a JWK, which is what another implementation reads.
    pub(super) fn jwk(&self) -> Value {
        json!({ "kty": "OKP", "crv": "Ed25519", "x": self.public_key_b64 })
    }

    pub(super) fn kid(&self) -> &str {
        &self.kid
    }

    /// A compact JWS over `payload`, with this issuer's `kid` and `typ`.
    pub(super) fn sign(&self, payload: &Value) -> String {
        self.sign_typed(payload, TYP)
    }

    pub(super) fn sign_typed(&self, payload: &Value, typ: &str) -> String {
        sign_typed(&self.store, KEY_ID, payload, &self.kid, Some(typ), None)
            .expect("the key store signs")
    }

    /// `JWT~D1~…~DN~`, signing `payload` and joining `disclosures` as given.
    ///
    /// Takes the disclosures as strings because the adversarial corpus needs to
    /// hand over ones no builder would make.
    pub(super) fn token(&self, payload: &Value, disclosures: &[String]) -> String {
        assemble(&self.sign(payload), disclosures)
    }
}

pub(super) fn assemble(jwt: &str, disclosures: &[String]) -> String {
    let mut out = String::from(jwt);
    for d in disclosures {
        out.push('~');
        out.push_str(d);
    }
    out.push('~');
    out
}

/// What this crate says about a token: accepted with this payload, or refused
/// for this reason.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Verdict {
    pub(super) payload: Option<Value>,
    pub(super) error: Option<String>,
}

impl Verdict {
    pub(super) fn accepts(&self) -> bool {
        self.payload.is_some()
    }

    pub(super) fn to_json(&self) -> Value {
        match (&self.payload, &self.error) {
            (Some(payload), _) => json!({ "accepts": true, "payload": payload }),
            (None, error) => json!({ "accepts": false, "error": error }),
        }
    }

    fn refused(why: impl std::fmt::Debug) -> Self {
        Self {
            payload: None,
            error: Some(format!("{why:?}")),
        }
    }
}

/// Parse, then process the Disclosures: RFC 9901 clause 7.1 without step 2's
/// signature check. The only verdict available for a token signed with an
/// algorithm this crate does not verify, such as the RFC's own ES256 examples.
pub(super) fn structure(token: &str) -> Verdict {
    match SdJwt::parse(token).map(|parsed| parsed.disclosed_payload()) {
        Ok(Ok(map)) => Verdict {
            payload: Some(Value::Object(map)),
            error: None,
        },
        Ok(Err(e)) | Err(e) => Verdict::refused(e),
    }
}

/// [`structure`], preceded by the signature check, in the order a caller of this
/// crate has to assemble for itself.
pub(super) fn ours(token: &str, public_key_b64: &str) -> Verdict {
    let Ok(parsed) = SdJwt::parse(token) else {
        return structure(token);
    };
    match verify_jws(parsed.jwt(), public_key_b64) {
        Ok(true) => structure(token),
        other => Verdict::refused(format!("signature: {other:?}")),
    }
}
