//! Key Binding, RFC 9901 clauses 4.1.2, 4.3 and 7.3: a presentation signed by the
//! key its Issuer named, so that whoever holds a copy of the token cannot replay
//! it as their own.
//!
//! Each role has one entry point. An Issuer names the Holder's key with
//! [`holder_key_confirmation`]. A Holder binds a presentation with
//! [`SdJwt::present_with_key_binding`]. A Verifier whose policy requires Key
//! Binding reads a token with [`SdJwt::disclosed_payload_with_key_binding`]
//! instead of [`SdJwt::disclosed_payload`]. Which of the two a Verifier calls is
//! its policy, and clause 7.3 step 1 says that choice MUST NOT depend on whether a
//! Key Binding JWT arrived, so neither looks at the token to decide.
//!
//! Ed25519 only, like the rest of this crate: the Holder key in `cnf` must be an
//! Ed25519 JWK, and the Key Binding JWT is signed with it. The RFC's own examples
//! bind with P-256 keys, so for those this crate checks the `sd_hash` and not the
//! signature.

use serde_json::{Map, Value, json};

use super::builder::SdJwt;
use super::disclosure::digest_of;
use super::error::SdJwtError;
use crate::jws::algorithm::KeyAlgorithm;
use crate::jws::signer::sign_with_header;
use crate::jws::verifier::{jwk_ed25519_x, verify_jws};
use crate::keystore::KeyEntry;

/// The `typ` of a Key Binding JWT, clause 4.3.
const KB_TYP: &str = "kb+jwt";

/// What a Holder binds a presentation to (clause 4.3): who it is for, the
/// challenge it answers, and when it was made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBinding {
    /// `aud`: the Verifier the presentation is for.
    pub audience: String,
    /// `nonce`: the Verifier's challenge for this transaction.
    pub nonce: String,
    /// `iat`, in seconds since the epoch.
    pub issued_at: i64,
}

/// What a Verifier whose policy requires Key Binding accepts (clause 7.3 step 5).
///
/// The time is supplied, not read: a verdict that depends on a hidden clock
/// cannot be reproduced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBindingPolicy {
    /// The `aud` the Key Binding JWT must name, this Verifier (step 5.f).
    pub audience: String,
    /// The `nonce` this Verifier issued for this transaction (step 5.f).
    pub nonce: String,
    /// Now, in seconds since the epoch.
    pub now: i64,
    /// The oldest a Key Binding JWT may be, in seconds (step 5.e).
    pub max_age: i64,
    /// How far clocks may disagree, in seconds: how far ahead of `now` an `iat`
    /// may lie, and the leeway given to an `exp` or `nbf` the JWT carries.
    pub leeway: i64,
}

/// The `cnf` value an Issuer puts in the payload to enable Key Binding for the
/// Holder whose Ed25519 public key is `holder_public_key` (clause 4.1.2): RFC
/// 7800's `jwk` confirmation method, which the clause suggests.
pub fn holder_key_confirmation(holder_public_key: &[u8]) -> Value {
    json!({ "jwk": KeyAlgorithm::Ed25519.public_key_jwk(holder_public_key) })
}

/// The Holder key a processed payload names: the `x` of an Ed25519 JWK in
/// `cnf.jwk`, and the `alg` the JWK declares, if it declares one.
fn holder_key(payload: &Map<String, Value>) -> Result<(String, Option<String>), SdJwtError> {
    let jwk = payload
        .get("cnf")
        .and_then(|cnf| cnf.get("jwk"))
        .ok_or(SdJwtError::NoHolderKey)?;
    let x = jwk_ed25519_x(jwk).ok_or(SdJwtError::UnsupportedHolderKey)?;
    let declared = match jwk.get("alg") {
        None => None,
        Some(alg) => {
            let alg = alg.as_str().ok_or(SdJwtError::UnsupportedHolderKey)?;
            if KeyAlgorithm::from_jose_alg(alg) != Some(KeyAlgorithm::Ed25519) {
                return Err(SdJwtError::UnsupportedHolderKey);
            }
            Some(alg.to_owned())
        }
    };
    Ok((x, declared))
}

/// Whether a `typ` names `kb+jwt`. RFC 7515 clause 4.1.9: media types compare
/// without regard to case, and a value with no `/` is read as if `application/`
/// came before it, so `application/kb+jwt` is the same type.
fn is_key_binding_typ(typ: &str) -> bool {
    let typ = typ.to_ascii_lowercase();
    typ == KB_TYP || typ.strip_prefix("application/") == Some(KB_TYP)
}

/// The header and claims of a compact JWT, read without checking anything.
fn decoded(jwt: &str) -> Option<(Map<String, Value>, Map<String, Value>)> {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let mut parts = jwt.split('.');
    let object = |part: Option<&str>| match serde_json::from_slice(&b64.decode(part?).ok()?) {
        Ok(Value::Object(map)) => Some(map),
        _ => None,
    };
    Some((object(parts.next())?, object(parts.next())?))
}

/// A NumericDate claim (RFC 7519 clause 2) as seconds, when it is one.
fn seconds(claims: &Map<String, Value>, name: &str) -> Option<f64> {
    claims
        .get(name)
        .and_then(Value::as_f64)
        .filter(|s| s.is_finite())
}

impl SdJwt {
    /// The `sd_hash` a Key Binding JWT carries for this SD-JWT (clause 4.3.1):
    /// the base64url SHA-256 of `<Issuer-signed JWT>~<D.1>~...~<D.N>~`, with the
    /// Disclosures as they travel and any Key Binding JWT left out.
    ///
    /// SHA-256 because it is the only hash this crate reads a token with, and the
    /// clause says the hash is the token's own.
    pub fn sd_hash(&self) -> String {
        let mut presented = String::from(self.jwt());
        for d in self.disclosures() {
            presented.push('~');
            presented.push_str(d.encoded());
        }
        presented.push('~');
        digest_of(&presented)
    }

    /// [`Self::present`], then bound to the Holder's key (clause 7.2 step 5).
    ///
    /// The Key Binding JWT carries `typ: kb+jwt` and the `alg` of `holder`, and
    /// nothing else in its header, since clause 4.3 says further parameters
    /// should be avoided. Its payload is `iat`, `aud`, `nonce` and the `sd_hash`
    /// of the presentation it is attached to.
    ///
    /// # Errors
    ///
    /// What [`Self::present`] refuses. [`SdJwtError::NoHolderKey`] when the
    /// presentation names no Holder key, which is also the case when `cnf` was
    /// concealed and not chosen: the Verifier could not find the key either.
    /// [`SdJwtError::UnsupportedHolderKey`] for a key that is not Ed25519, and
    /// [`SdJwtError::HolderKeyMismatch`] when `holder` is not the key `cnf`
    /// names.
    pub fn present_with_key_binding(
        &self,
        reveal_digests: &[&str],
        binding: &KeyBinding,
        holder: &KeyEntry,
    ) -> Result<Self, SdJwtError> {
        use base64::Engine;
        let presentation = self.present(reveal_digests)?;
        let payload = presentation.disclosed_payload()?;
        let (x, declared) = holder_key(&payload)?;
        let ours = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(holder.verifying_key.to_bytes());
        if x != ours {
            return Err(SdJwtError::HolderKeyMismatch);
        }

        // The `alg` a JWK declares is the one a Verifier holds the signature to,
        // so it is the one written.
        let alg = declared.unwrap_or_else(|| holder.algorithm.jose_alg().to_owned());
        let mut header = Map::new();
        header.insert("alg".to_owned(), Value::String(alg));
        header.insert("typ".to_owned(), Value::String(KB_TYP.to_owned()));
        let claims = json!({
            "iat": binding.issued_at,
            "aud": binding.audience,
            "nonce": binding.nonce,
            "sd_hash": presentation.sd_hash(),
        });
        let jwt = sign_with_header(holder, &header, claims.to_string().as_bytes());
        Ok(presentation.bound(jwt))
    }

    /// [`Self::disclosed_payload`], for a Verifier whose policy requires Key
    /// Binding: clause 7.3, steps 2, 4 and 5, in that order.
    ///
    /// The Holder key is taken from the processed payload's `cnf` (step 5.a), so
    /// a `cnf` that is selectively disclosable has to have been disclosed. The
    /// signature is checked against it with this crate's JWS verifier, which
    /// refuses `none`, any algorithm but Ed25519 and any `crit` (step 5.b), and
    /// an `alg` other than the one the JWK declares.
    ///
    /// # Errors
    ///
    /// [`SdJwtError::KeyBindingMissing`] for a token without a Key Binding JWT;
    /// whatever [`Self::disclosed_payload`] refuses;
    /// [`SdJwtError::NoHolderKey`] and [`SdJwtError::UnsupportedHolderKey`] for
    /// the key; and [`SdJwtError::KeyBindingRejected`] naming the check of step 5
    /// the Key Binding JWT failed.
    pub fn disclosed_payload_with_key_binding(
        &self,
        policy: &KeyBindingPolicy,
    ) -> Result<Map<String, Value>, SdJwtError> {
        use SdJwtError::KeyBindingRejected as Rejected;

        let kb = self
            .key_binding_jwt()
            .ok_or(SdJwtError::KeyBindingMissing)?;
        let payload = self.disclosed_payload()?;
        let (x, declared) = holder_key(&payload)?;

        let (header, claims) = decoded(kb).ok_or(Rejected("the JWT"))?;
        let alg = header.get("alg").and_then(Value::as_str);
        if declared.is_some() && declared.as_deref() != alg {
            return Err(Rejected("alg"));
        }
        if !verify_jws(kb, &x).unwrap_or(false) {
            return Err(Rejected("the signature"));
        }
        if !header
            .get("typ")
            .and_then(Value::as_str)
            .is_some_and(is_key_binding_typ)
        {
            return Err(Rejected("typ"));
        }

        #[expect(
            clippy::cast_precision_loss,
            reason = "seconds since the epoch are far inside f64's exact integers"
        )]
        let (now, max_age, leeway) = (
            policy.now as f64,
            policy.max_age as f64,
            policy.leeway as f64,
        );
        let iat = seconds(&claims, "iat").ok_or(Rejected("iat"))?;
        if iat < now - max_age || iat > now + leeway {
            return Err(Rejected("iat"));
        }
        if claims.get("nonce").and_then(Value::as_str) != Some(policy.nonce.as_str()) {
            return Err(Rejected("nonce"));
        }
        // A single string, clause 4.3: an array naming this Verifier is not one.
        if claims.get("aud").and_then(Value::as_str) != Some(policy.audience.as_str()) {
            return Err(Rejected("aud"));
        }
        if claims.get("sd_hash").and_then(Value::as_str) != Some(self.sd_hash().as_str()) {
            return Err(Rejected("sd_hash"));
        }
        // Step 5.h: valid in all other respects, which for RFC 7519 is the two
        // other time claims, when present.
        if claims.contains_key("exp")
            && !seconds(&claims, "exp").is_some_and(|exp| now < exp + leeway)
        {
            return Err(Rejected("exp"));
        }
        if claims.contains_key("nbf")
            && !seconds(&claims, "nbf").is_some_and(|nbf| now + leeway >= nbf)
        {
            return Err(Rejected("nbf"));
        }
        Ok(payload)
    }
}
