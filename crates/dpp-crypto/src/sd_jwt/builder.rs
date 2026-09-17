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
) -> Result<(Map<String, Value>, Vec<Disclosure>), SdJwtError> {
    let mut kept = Map::new();
    let mut disclosures = Vec::new();

    for (name, value) in object {
        if conceal_if(name) {
            disclosures.push(Disclosure::new(name.clone(), value.clone())?);
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

    Ok((kept, disclosures))
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

    /// The disclosures currently attached, **unauthenticated**.
    ///
    /// 🚨 A disclosure here has not been matched against the signed `_sd`
    /// digests. [`Self::parse`] reads the serialisation; it does not check that
    /// what arrived is what the issuer committed to. So anyone can append a
    /// forged disclosure to an untouched, validly signed credential, and it
    /// comes back from this method looking exactly like a real one — the
    /// signature still verifies, because the signature never covered this list.
    ///
    /// Treat every value here as attacker-supplied until
    /// [`Self::disclosed_payload`] has succeeded, and read the claim from *its*
    /// output rather than from here. That is the call which recomputes each
    /// digest and admits only the disclosures the issuer signed over.
    ///
    /// This method exists for inspection — counting what travelled, rendering a
    /// presentation, choosing what to reveal next — none of which is a trust
    /// decision.
    pub fn disclosures(&self) -> &[Disclosure] {
        &self.disclosures
    }

    /// Whether a key-binding JWT is attached. Its contents are not interpreted.
    pub fn has_key_binding(&self) -> bool {
        self.key_binding_jwt.is_some()
    }

    /// The digests of every disclosure carrying `claim_name`.
    ///
    /// Plural on purpose. One claim name can occur at several places in a
    /// credential — RFC 9901 clause 9.3 says so explicitly, which is why each
    /// occurrence gets its own salt — so "the disclosure for `name`" is not
    /// always a single thing. A caller that gets two digests back is being told
    /// that revealing "name" is an ambiguous instruction, and has to choose.
    pub fn digests_for_claim(&self, claim_name: &str) -> Vec<String> {
        self.disclosures
            .iter()
            .filter(|d| d.claim_name() == claim_name)
            .map(Disclosure::digest)
            .collect()
    }

    /// A presentation carrying only the disclosures whose digests are listed.
    ///
    /// The signed JWT is forwarded unchanged, so the result still verifies
    /// against the issuer's key. Disclosures not listed keep their digests in
    /// the token and their values do not travel — which is the whole point, and
    /// is asserted over the presentation bytes in this module's tests rather
    /// than inferred from this being the intent.
    ///
    /// # Why digests and not claim names
    ///
    /// A digest identifies one disclosure; a claim name may identify several.
    /// Selecting by name meant that revealing a claim also revealed every
    /// *other* disclosure that happened to share its name — a value the holder
    /// did not choose to release, leaving by the door built to stop exactly
    /// that. Use [`Self::digests_for_claim`] to go from a name to the digests,
    /// where the ambiguity is visible instead of silently resolved.
    ///
    /// Digests that match nothing are ignored rather than rejected: asking for
    /// a claim that was issued in cleartext is asking for something the
    /// presentation already carries.
    pub fn present(&self, reveal_digests: &[&str]) -> Self {
        Self {
            jwt: self.jwt.clone(),
            disclosures: self
                .disclosures
                .iter()
                .filter(|d| reveal_digests.contains(&d.digest().as_str()))
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
    ///
    /// [`SdJwtError::DuplicateDigest`] if one digest appears twice — whether in
    /// the disclosures supplied or in the token's own `_sd` arrays. Clause 4.1:
    /// *"The same digest value MUST NOT appear more than once in the SD-JWT."*
    pub fn disclosed_payload(&self) -> Result<Map<String, Value>, SdJwtError> {
        let payload = decode_jwt_payload(&self.jwt)?;

        if let Some(alg) = payload.get(SD_ALG_CLAIM) {
            let named = alg.as_str().unwrap_or_default();
            if named != SD_HASH_ALG {
                return Err(SdJwtError::UnsupportedHashAlg(named.to_owned()));
            }
        }

        // Built one at a time rather than `collect`ed, because a map silently
        // keeps the last of two equal keys — so collecting would *implement*
        // the duplicate-tolerance clause 4.1 forbids, and hide it behind a
        // disclosure count that still added up.
        let mut by_digest: std::collections::HashMap<String, &Disclosure> =
            std::collections::HashMap::new();
        for d in &self.disclosures {
            let digest = digest_of(d.encoded());
            if by_digest.insert(digest.clone(), d).is_some() {
                return Err(SdJwtError::DuplicateDigest(digest));
            }
        }

        let mut used = 0usize;
        let mut seen_digests = std::collections::HashSet::new();
        let mut object = Value::Object(payload);
        substitute(&mut object, &by_digest, &mut used, &mut seen_digests)?;

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
    seen_digests: &mut std::collections::HashSet<String>,
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
                // Clause 4.1's "MUST NOT appear more than once" is about the
                // whole SD-JWT, not one `_sd` array, so the set spans the walk.
                // Checked for *every* embedded digest, matched or not: an
                // unmatched digest repeated across two objects is still a
                // malformed token, and removing the arrays would otherwise make
                // it indistinguishable from a well-formed one.
                if !seen_digests.insert(digest.to_owned()) {
                    return Err(SdJwtError::DuplicateDigest(digest.to_owned()));
                }
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
                substitute(v, by_digest, used, seen_digests)?;
            }
            Ok(())
        }
        Value::Array(items) => {
            for item in items {
                substitute(item, by_digest, used, seen_digests)?;
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
