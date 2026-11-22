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
//! The Holder key in `cnf` is an Ed25519 (`OKP`) JWK, or an EC (`EC`) JWK on P-256,
//! P-384 or P-521, and the Key Binding JWT is held to that key's algorithm. The
//! RFC's own examples bind with P-256 keys, and their Key Binding JWTs verify here.

use serde_json::{Map, Value, json};

use super::builder::SdJwt;
use super::disclosure::digest_of;
use super::error::SdJwtError;
use crate::jws::algorithm::KeyAlgorithm;
use crate::jws::public_key::PublicKey;
use crate::jws::signer::sign_with_header;
use crate::jws::verifier::verify_jws;
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
    ///
    /// Unsigned, as is `leeway`: a negative window would refuse every honest
    /// token for a reason that names the token and not the policy.
    pub max_age: u32,
    /// How far clocks may disagree, in seconds: how far ahead of `now` an `iat`
    /// may lie, and the leeway given to an `exp` or `nbf` the JWT carries.
    pub leeway: u32,
}

/// The `cnf` value an Issuer puts in the payload to enable Key Binding for the
/// Holder whose public key is `holder` (clause 4.1.2): RFC 7800's `jwk`
/// confirmation method, which the clause suggests.
pub fn holder_key_confirmation(holder: &PublicKey) -> Value {
    json!({ "jwk": holder.jwk() })
}

/// The Holder key a processed payload names: the Ed25519 or EC JWK (P-256, P-384 or
/// P-521) in `cnf.jwk`, and the `alg` the JWK declares, if it declares one. A declared
/// `alg` must be the key's own.
fn holder_key(payload: &Map<String, Value>) -> Result<(PublicKey, Option<String>), SdJwtError> {
    let cnf = payload.get("cnf").ok_or(SdJwtError::NoHolderKey)?;
    let jwk = cnf.get("jwk").ok_or(SdJwtError::NoHolderKey)?;
    // RFC 7800 §3.1: the `cnf` value represents a single proof-of-possession
    // key, so `jwk` is refused beside `jwe` or `jku`, which name the key another way.
    // `kid` is not one of the three and stays allowed.
    if cnf.get("jwe").is_some() || cnf.get("jku").is_some() {
        return Err(SdJwtError::UnsupportedHolderKey);
    }
    let key = PublicKey::from_jwk(jwk).ok_or(SdJwtError::UnsupportedHolderKey)?;
    let declared = match jwk.get("alg") {
        None => None,
        Some(alg) => {
            let alg = alg.as_str().ok_or(SdJwtError::UnsupportedHolderKey)?;
            if KeyAlgorithm::from_jose_alg(alg) != Some(key.algorithm()) {
                return Err(SdJwtError::UnsupportedHolderKey);
            }
            Some(alg.to_owned())
        }
    };
    Ok((key, declared))
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
    /// [`SdJwtError::UnsupportedHolderKey`] for a key that is none of Ed25519, P-256,
    /// P-384 and P-521, and
    /// [`SdJwtError::HolderKeyMismatch`] when `holder` is not the key `cnf`
    /// names, and [`SdJwtError::KeyRevoked`] when `holder` is revoked.
    pub fn present_with_key_binding(
        &self,
        reveal_digests: &[&str],
        binding: &KeyBinding,
        holder: &KeyEntry,
    ) -> Result<Self, SdJwtError> {
        let presentation = self.present(reveal_digests)?;
        let payload = presentation.disclosed_payload()?;
        let (named, declared) = holder_key(&payload)?;
        if named != holder.public_key() {
            return Err(SdJwtError::HolderKeyMismatch);
        }

        // The `alg` a JWK declares is the one a Verifier holds the signature to,
        // so it is the one written. Without a declaration, the confirmed key's own
        // algorithm is, not the `algorithm` field of the entry, which a caller can set.
        let alg = declared.unwrap_or_else(|| named.algorithm().jose_alg().to_owned());
        let mut header = Map::new();
        header.insert("alg".to_owned(), Value::String(alg));
        header.insert("typ".to_owned(), Value::String(KB_TYP.to_owned()));
        let claims = json!({
            "iat": binding.issued_at,
            "aud": binding.audience,
            "nonce": binding.nonce,
            "sd_hash": presentation.sd_hash(),
        });
        let jwt = sign_with_header(holder, &header, claims.to_string().as_bytes())?;
        Ok(presentation.bound(jwt))
    }

    /// [`Self::disclosed_payload`], for a Verifier whose policy requires Key
    /// Binding: clause 7.3, steps 2, 4 and 5, in that order.
    ///
    /// The Holder key is taken from the processed payload's `cnf` (step 5.a), so
    /// a `cnf` that is selectively disclosable has to have been disclosed. The
    /// signature is checked against it with this crate's JWS verifier, which
    /// refuses `none`, an algorithm other than the key's own and any `crit` (step 5.b), and
    /// an `alg` other than the one the JWK declares. A header that sets `b64` to `false` is
    /// refused before any of these, since RFC 7797 §7 forbids it in a JWT.
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
        let (key, declared) = holder_key(&payload)?;

        let (header, claims) = decoded(kb).ok_or(Rejected("the JWT"))?;
        // A Key Binding JWT is a JWT (clause 4.3), and RFC 7797 §7: "JSON Web Tokens [JWT]
        // MUST NOT use "b64" with a "false" value".
        if header.get("b64") == Some(&Value::Bool(false)) {
            return Err(Rejected("b64"));
        }
        let alg = header.get("alg").and_then(Value::as_str);
        if declared.is_some() && declared.as_deref() != alg {
            return Err(Rejected("alg"));
        }
        if !verify_jws(kb, &key).unwrap_or(false) {
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
            f64::from(policy.max_age),
            f64::from(policy.leeway),
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
