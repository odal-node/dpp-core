//! The issuer key the corpus is signed with, how a token is assembled, and the
//! verdict this crate gives one.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::{Value, json};

use crate::jws::sign_typed;
use crate::jws::verifier::verify_jws;
use crate::keystore::{KeyEntry, KeyStore};
use crate::sd_jwt::{KeyBindingPolicy, SdJwt};
use crate::test_support::temp_store;

const KEY_ID: &str = "sdjwt-oracle-issuer";

/// The `typ` the RFC's own examples carry. The RFC does not require one; an
/// issuer that sets it names the kind of token explicitly (clause 9.11).
pub(crate) const TYP: &str = "example+sd-jwt";

/// An Ed25519 issuer, the algorithm this crate signs with.
pub(crate) struct Issuer {
    store: KeyStore,
    kid: String,
    /// The raw public key, base64url, the form [`verify_jws`] takes.
    pub(crate) public_key_b64: String,
}

impl Issuer {
    pub(crate) fn new() -> Self {
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
    pub(crate) fn jwk(&self) -> Value {
        json!({ "kty": "OKP", "crv": "Ed25519", "x": self.public_key_b64 })
    }

    pub(crate) fn kid(&self) -> &str {
        &self.kid
    }

    /// The issuer's own key pair, for a Key Binding JWT signed with the wrong key.
    pub(crate) fn key(&self) -> KeyEntry {
        self.store
            .load_key(KEY_ID)
            .expect("the key was just generated")
    }

    /// A compact JWS over `payload`, with this issuer's `kid` and `typ`.
    pub(crate) fn sign(&self, payload: &Value) -> String {
        self.sign_typed(payload, TYP)
    }

    pub(crate) fn sign_typed(&self, payload: &Value, typ: &str) -> String {
        sign_typed(&self.store, KEY_ID, payload, &self.kid, Some(typ), None)
            .expect("the key store signs")
    }

    /// `JWT~D1~…~DN~`, signing `payload` and joining `disclosures` as given.
    ///
    /// Takes the disclosures as strings because the adversarial corpus needs to
    /// hand over ones no builder would make.
    pub(crate) fn token(&self, payload: &Value, disclosures: &[String]) -> String {
        assemble(&self.sign(payload), disclosures)
    }
}

pub(crate) fn assemble(jwt: &str, disclosures: &[String]) -> String {
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
pub(crate) struct Verdict {
    pub(crate) payload: Option<Value>,
    pub(crate) error: Option<String>,
}

impl Verdict {
    pub(crate) fn to_json(&self) -> Value {
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
/// algorithm this crate does not verify, such as an ES512 token.
pub(crate) fn structure(token: &str) -> Verdict {
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
pub(crate) fn ours(token: &str, public_key_b64: &str) -> Verdict {
    let Ok(parsed) = SdJwt::parse(token) else {
        return structure(token);
    };
    match ed25519_key(public_key_b64) {
        Some(key) => match verify_jws(parsed.jwt(), &key) {
            Ok(()) => structure(token),
            other => Verdict::refused(format!("signature: {other:?}")),
        },
        None => Verdict::refused(NOT_A_KEY),
    }
}

/// Why a token is refused when its issuer key does not decode. The token is
/// refused, as it was when `verify_jws` rejected the key itself.
pub(crate) const NOT_A_KEY: &str = "signature: the issuer key is not an Ed25519 public key";

/// An Ed25519 public key given as base64url, the shape the oracle files use.
/// `None` when it is not one.
pub(crate) fn ed25519_key(public_key_b64: &str) -> Option<crate::jws::PublicKey> {
    let raw = URL_SAFE_NO_PAD.decode(public_key_b64).ok()?;
    crate::jws::PublicKey::from_bytes(crate::jws::KeyAlgorithm::Ed25519, &raw).ok()
}

/// A Holder: the Ed25519 key an Issuer names in `cnf` and binds a presentation
/// with.
pub(crate) fn holder(label: &str) -> KeyEntry {
    temp_store(label, "holder")
        .load_key("holder")
        .expect("the key was just generated")
}

/// [`structure`] for a Verifier whose policy requires Key Binding: clause 7.1,
/// then clause 7.3, without the Issuer's signature.
pub(crate) fn bound_structure(token: &str, policy: &KeyBindingPolicy) -> Verdict {
    match SdJwt::parse(token).map(|parsed| parsed.disclosed_payload_with_key_binding(policy)) {
        Ok(Ok(map)) => Verdict {
            payload: Some(Value::Object(map)),
            error: None,
        },
        Ok(Err(e)) | Err(e) => Verdict::refused(e),
    }
}

/// [`bound_structure`], preceded by the Issuer's signature check.
pub(crate) fn ours_bound(token: &str, public_key_b64: &str, policy: &KeyBindingPolicy) -> Verdict {
    let Ok(parsed) = SdJwt::parse(token) else {
        return bound_structure(token, policy);
    };
    match ed25519_key(public_key_b64) {
        Some(key) => match verify_jws(parsed.jwt(), &key) {
            Ok(()) => bound_structure(token, policy),
            other => Verdict::refused(format!("signature: {other:?}")),
        },
        None => Verdict::refused(NOT_A_KEY),
    }
}
