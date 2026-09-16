//! Concealing claims, and reading a concealed token back.

use serde_json::{Map, Value};

use super::disclosure::{Disclosure, SD_HASH_ALG, digest_of};
use super::error::SdJwtError;

/// The claim carrying an object's digests, RFC 9901 clause 4.2.4.1.
pub(crate) const SD_CLAIM: &str = "_sd";
/// The claim naming the hash algorithm, RFC 9901 clause 4.1.1.
pub(crate) const SD_ALG_CLAIM: &str = "_sd_alg";

/// Replace the named members of `object` with digests, returning the rewritten
/// object and the disclosures that reopen it.
///
/// `conceal_if` decides, per member name, whether that member is hidden. It is a
/// predicate rather than a list because the caller classifying a passport is
/// walking a tree and already knows the answer at each node; handing it a list
/// would make it build one.
///
/// Members not selected are left in cleartext. Nested objects are **not**
/// descended into — a caller that wants a nested member concealed calls this on
/// that object too, which is what keeps the digest attached to the object the
/// claim actually sits in, as clause 4.2.4.1 requires.
///
/// `_sd` is omitted entirely when nothing is concealed, per clause 4.2.4.1's
/// note that an issuer may omit it to save space; `_sd_alg` is added only at the
/// top level by [`build_payload`].
pub fn conceal(
    object: &Map<String, Value>,
    mut conceal_if: impl FnMut(&str) -> bool,
) -> (Map<String, Value>, Vec<Disclosure>) {
    let mut kept = Map::new();
    let mut disclosures = Vec::new();

    for (name, value) in object {
        if conceal_if(name) {
            disclosures.push(Disclosure::new(name.clone(), value.clone()));
        } else {
            kept.insert(name.clone(), value.clone());
        }
    }

    if !disclosures.is_empty() {
        // Clause 4.2.4.1: "The Issuer MUST hide the original order of the claims
        // in the array." Sorting satisfies it deterministically — the clause
        // names sorting alphanumerically as an acceptable method and says the
        // precise one does not matter "as long as it does not depend on the
        // original order of elements". Deterministic beats random here because
        // it makes the issued bytes reproducible from the disclosures for a
        // given salt set, which is what lets a test pin them.
        let mut digests: Vec<Value> = disclosures
            .iter()
            .map(|d| Value::String(d.digest()))
            .collect();
        digests.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        kept.insert(SD_CLAIM.to_owned(), Value::Array(digests));
    }

    (kept, disclosures)
}

/// A parsed SD-JWT: the issuer-signed JWT, verbatim, plus the disclosures that
/// travelled with it.
///
/// The JWT is kept as the exact string it arrived as. A presentation forwards it
/// untouched — the holder never re-signs, so any re-serialisation here would
/// invalidate a signature the holder cannot remake.
#[derive(Debug, Clone)]
pub struct SdJwt {
    jwt: String,
    disclosures: Vec<Disclosure>,
    key_binding_jwt: Option<String>,
}

impl SdJwt {
    /// Assemble from an already-signed JWT and its disclosures.
    pub fn new(jwt: String, disclosures: Vec<Disclosure>) -> Self {
        Self {
            jwt,
            disclosures,
            key_binding_jwt: None,
        }
    }

    /// Read the combined format of RFC 9901 clause 4:
    /// `<JWT>~<Disclosure>~...~<optional KB-JWT>`.
    ///
    /// The trailing `~` is required by the format and is what distinguishes
    /// "no key binding" from a truncated token.
    pub fn parse(serialised: &str) -> Result<Self, SdJwtError> {
        let segments: Vec<&str> = serialised.split('~').collect();
        let [jwt, rest @ .., last] = segments.as_slice() else {
            return Err(SdJwtError::NotAnSdJwt);
        };
        let disclosures = rest
            .iter()
            .map(|s| Disclosure::parse(s))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            jwt: (*jwt).to_owned(),
            disclosures,
            key_binding_jwt: (!last.is_empty()).then(|| (*last).to_owned()),
        })
    }

    /// The combined format, for transmission.
    pub fn serialise(&self) -> String {
        let mut out = String::from(&self.jwt);
        for d in &self.disclosures {
            out.push('~');
            out.push_str(d.encoded());
        }
        out.push('~');
        if let Some(kb) = &self.key_binding_jwt {
            out.push_str(kb);
        }
        out
    }

    /// The issuer-signed JWT, for [`crate::jws::verifier::verify_jws`].
    pub fn jwt(&self) -> &str {
        &self.jwt
    }

    /// The disclosures currently attached.
    pub fn disclosures(&self) -> &[Disclosure] {
        &self.disclosures
    }

    /// Whether a key-binding JWT is attached. Its contents are not interpreted.
    pub fn has_key_binding(&self) -> bool {
        self.key_binding_jwt.is_some()
    }

    /// A presentation carrying only the claims named in `reveal`.
    ///
    /// The signed JWT is forwarded unchanged, so the result still verifies
    /// against the issuer's key. Claims not named keep their digests in the
    /// token and their values do not travel — which is the whole point, and is
    /// asserted over the presentation bytes in this module's tests rather than
    /// inferred from this being the intent.
    ///
    /// Names that are not disclosable here are ignored rather than rejected: a
    /// holder asking for a claim that was issued in cleartext is asking for
    /// something it already has.
    pub fn present(&self, reveal: &[&str]) -> Self {
        Self {
            jwt: self.jwt.clone(),
            disclosures: self
                .disclosures
                .iter()
                .filter(|d| reveal.contains(&d.claim_name()))
                .cloned()
                .collect(),
            key_binding_jwt: self.key_binding_jwt.clone(),
        }
    }

    /// The payload with every supplied disclosure substituted back in, and every
    /// `_sd` / `_sd_alg` artefact removed.
    ///
    /// This is RFC 9901 clause 7.1's processing, minus the signature check,
    /// which is [`crate::jws::verifier::verify_jws`] and is the caller's to run.
    ///
    /// # Errors
    ///
    /// [`SdJwtError::UnusedDisclosures`] if any disclosure's digest is absent
    /// from the token. That is the tamper signal: change a disclosed value and
    /// its digest stops matching, so the disclosure goes unused and the whole
    /// credential is refused rather than the claim quietly vanishing.
    pub fn disclosed_payload(&self) -> Result<Map<String, Value>, SdJwtError> {
        let payload = decode_jwt_payload(&self.jwt)?;

        if let Some(alg) = payload.get(SD_ALG_CLAIM) {
            let named = alg.as_str().unwrap_or_default();
            if named != SD_HASH_ALG {
                return Err(SdJwtError::UnsupportedHashAlg(named.to_owned()));
            }
        }

        let by_digest: std::collections::HashMap<String, &Disclosure> = self
            .disclosures
            .iter()
            .map(|d| (digest_of(d.encoded()), d))
            .collect();

        let mut used = 0usize;
        let mut object = Value::Object(payload);
        substitute(&mut object, &by_digest, &mut used)?;

        if used != by_digest.len() {
            return Err(SdJwtError::UnusedDisclosures(by_digest.len() - used));
        }

        match object {
            Value::Object(map) => Ok(map),
            _ => Err(SdJwtError::PayloadNotAnObject),
        }
    }
}

/// Walk a value, replacing every `_sd` digest that has a matching disclosure and
/// stripping the mechanism's own claims as it goes.
fn substitute(
    value: &mut Value,
    by_digest: &std::collections::HashMap<String, &Disclosure>,
    used: &mut usize,
) -> Result<(), SdJwtError> {
    match value {
        Value::Object(map) => {
            let digests = match map.remove(SD_CLAIM) {
                Some(Value::Array(items)) => items,
                // A non-array `_sd` is not a shape clause 4.2.4.1 defines. It is
                // dropped rather than rejected: it conceals nothing, so it can
                // hide nothing, and refusing would reject tokens over a claim
                // that carries no information.
                _ => Vec::new(),
            };
            map.remove(SD_ALG_CLAIM);

            for digest in digests {
                let Some(digest) = digest.as_str() else {
                    continue;
                };
                if let Some(d) = by_digest.get(digest) {
                    if map.contains_key(d.claim_name()) {
                        return Err(SdJwtError::ClaimCollision(d.claim_name().to_owned()));
                    }
                    map.insert(d.claim_name().to_owned(), d.claim_value().clone());
                    *used += 1;
                }
            }

            // Descend after substituting, so a disclosure whose value is itself
            // an object carrying `_sd` is opened too.
            for (_, v) in map.iter_mut() {
                substitute(v, by_digest, used)?;
            }
            Ok(())
        }
        Value::Array(items) => {
            for item in items {
                substitute(item, by_digest, used)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn decode_jwt_payload(jwt: &str) -> Result<Map<String, Value>, SdJwtError> {
    use base64::Engine;
    let payload_b64 = jwt.split('.').nth(1).ok_or(SdJwtError::MalformedJwt)?;
    if jwt.split('.').count() != 3 {
        return Err(SdJwtError::MalformedJwt);
    }
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|_| SdJwtError::MalformedJwt)?;
    match serde_json::from_slice(&bytes) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => Err(SdJwtError::PayloadNotAnObject),
        Err(_) => Err(SdJwtError::MalformedJwt),
    }
}

/// Build a signable SD-JWT payload from an object [`conceal`] has rewritten.
///
/// Adds `_sd_alg` when anything was concealed and leaves it off otherwise, since
/// clause 4.1.1 only requires the claim where selectively disclosable claims
/// exist. It goes at the **top level of the payload only**, once, however deeply
/// the concealment reaches — which is why this is a separate step from
/// [`conceal`] rather than something `conceal` could do for itself.
pub fn build_payload(mut object: Map<String, Value>, concealed_any: bool) -> Value {
    if concealed_any {
        object.insert(
            SD_ALG_CLAIM.to_owned(),
            Value::String(SD_HASH_ALG.to_owned()),
        );
    }
    Value::Object(object)
}
