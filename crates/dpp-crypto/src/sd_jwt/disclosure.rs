//! Disclosures — the `[salt, claim_name, claim_value]` triples of RFC 9901 clause
//! 4.2.1 for an object property, and the `[salt, value]` pairs of clause 4.2.2 for
//! an array element.

use base64::Engine;
use serde_json::Value;
use sha2::{Digest, Sha256};

use rand::Rng;

use super::error::DisclosureError;

/// The hash algorithm named in `_sd_alg`, from the IANA "Named Information Hash
/// Algorithm" registry as RFC 9901 clause 4.1.1 requires.
pub const SD_HASH_ALG: &str = "sha-256";

/// Length of the random portion of a salt, in bytes.
///
/// RFC 9901 clause 9.3 gives 128 bits as the RECOMMENDED minimum and this takes
/// it exactly. Longer would cost credential size — every disclosure carries its
/// salt base64url-encoded — for no stated benefit.
const SALT_BYTES: usize = 16;

/// Claim names RFC 9901 clause 4.2.1 forbids a disclosure from carrying,
/// because they are the mechanism's own keys.
const RESERVED: [&str; 2] = ["_sd", "..."];

fn b64() -> base64::engine::general_purpose::GeneralPurpose {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
}

/// One selectively disclosable claim, together with the exact string its digest
/// was computed over.
///
/// Two kinds, told apart by the array the RFC defines for each. An **object
/// property** is `[salt, name, value]` (clause 4.2.1) and its digest goes in an
/// `_sd` array. An **array element** is `[salt, value]` (clause 4.2.2): it has no
/// name, because its place is the position of the placeholder its digest stands
/// in for, and that digest goes in the array itself as `{"...": "<digest>"}`
/// (clause 4.2.4.2).
///
/// The encoded form is stored rather than recomputed because **the digest is
/// over the string, not over the array**. Re-serialising `[salt, name, value]`
/// is not guaranteed to reproduce the bytes an issuer hashed — a different
/// number formatting or key order in a nested object value would change the
/// digest and break verification for a credential that was never tampered with.
/// Whatever produced this value, the bytes that produced the digest are kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disclosure {
    salt: String,
    /// `None` for an array element, which has no name.
    claim_name: Option<String>,
    claim_value: Value,
    encoded: String,
}

impl Disclosure {
    /// Create a disclosure for `claim_name`, with a fresh salt.
    ///
    /// The salt is 128 bits from the OS CSPRNG, drawn independently per call.
    /// RFC 9901 clause 9.3 requires a new salt **for each claim, including when
    /// the same claim name occurs at different places in the structure** — so
    /// this takes no cache and offers no way to reuse one. A reused salt across
    /// two credentials for the same field is a correlation handle.
    /// # Errors
    ///
    /// [`DisclosureError::ReservedClaimName`] for `_sd` or `...`. Clause 4.2.1
    /// forbids them, and [`Disclosure::parse`] refuses them — so without this
    /// check a constructor could build a value that this crate's own parser
    /// rejects, and the disclosure would fail only at the verifier.
    pub fn new(claim_name: impl Into<String>, claim_value: Value) -> Result<Self, DisclosureError> {
        let mut salt_bytes = [0u8; SALT_BYTES];
        crate::os_rng().fill_bytes(&mut salt_bytes);
        Self::with_salt(b64().encode(salt_bytes), claim_name, claim_value)
    }

    /// Create a disclosure with a caller-supplied salt.
    ///
    /// Exists for tests and for re-encoding a disclosure whose salt is already
    /// fixed. Production issuance uses [`Disclosure::new`], which is the only
    /// path that guarantees the clause 9.3 property.
    ///
    /// # Errors
    ///
    /// [`DisclosureError::ReservedClaimName`], on the same terms as
    /// [`Disclosure::new`].
    pub fn with_salt(
        salt: String,
        claim_name: impl Into<String>,
        claim_value: Value,
    ) -> Result<Self, DisclosureError> {
        let claim_name = claim_name.into();
        if RESERVED.contains(&claim_name.as_str()) {
            return Err(DisclosureError::ReservedClaimName);
        }
        // RFC 9901 clause 4.2.1: base64url of the UTF-8 bytes of the JSON array.
        let encoded = b64().encode(
            Value::Array(vec![
                Value::String(salt.clone()),
                Value::String(claim_name.clone()),
                claim_value.clone(),
            ])
            .to_string(),
        );
        Ok(Self {
            salt,
            claim_name: Some(claim_name),
            claim_value,
            encoded,
        })
    }

    /// Create a disclosure for an array element, with a fresh salt.
    ///
    /// Clause 4.2.2: the element can be any JSON value. The salt is drawn the same
    /// way as for a property and for the same reason (clause 9.3), once per
    /// element: two equal elements of one array get two salts, so their
    /// Disclosures, and the digests that hide them, differ.
    pub fn element(claim_value: Value) -> Self {
        let mut salt_bytes = [0u8; SALT_BYTES];
        crate::os_rng().fill_bytes(&mut salt_bytes);
        Self::element_with_salt(b64().encode(salt_bytes), claim_value)
    }

    /// Create an array-element disclosure with a caller-supplied salt, on the same
    /// terms as [`Disclosure::with_salt`].
    pub fn element_with_salt(salt: String, claim_value: Value) -> Self {
        let encoded = b64().encode(
            Value::Array(vec![Value::String(salt.clone()), claim_value.clone()]).to_string(),
        );
        Self {
            salt,
            claim_name: None,
            claim_value,
            encoded,
        }
    }

    /// Read a disclosure produced elsewhere, keeping its original bytes.
    ///
    /// Three elements is an object property and two is an array element. Any other
    /// length, or a salt or claim name that is not a string, is refused.
    pub fn parse(encoded: &str) -> Result<Self, DisclosureError> {
        let bytes = b64()
            .decode(encoded)
            .map_err(|_| DisclosureError::NotBase64)?;
        let parsed: Value = serde_json::from_slice(&bytes).map_err(|_| DisclosureError::NotJson)?;
        let Some(items) = parsed.as_array() else {
            return Err(DisclosureError::NotATripleOrPair);
        };
        let (salt, claim_name, value) = match items.as_slice() {
            [salt, name, value] => (salt, Some(name), value),
            [salt, value] => (salt, None, value),
            _ => return Err(DisclosureError::NotATripleOrPair),
        };
        let Some(salt) = salt.as_str() else {
            return Err(DisclosureError::NotATripleOrPair);
        };
        let claim_name = match claim_name {
            None => None,
            Some(name) => {
                let Some(name) = name.as_str() else {
                    return Err(DisclosureError::NotATripleOrPair);
                };
                if RESERVED.contains(&name) {
                    return Err(DisclosureError::ReservedClaimName);
                }
                Some(name.to_owned())
            }
        };
        Ok(Self {
            salt: salt.to_owned(),
            claim_name,
            claim_value: value.clone(),
            encoded: encoded.to_owned(),
        })
    }

    /// The base64url string that travels in the credential.
    pub fn encoded(&self) -> &str {
        &self.encoded
    }

    /// The salt. Never leaves the issuer except to the holder — RFC 9901
    /// clause 9.3 makes that a `MUST NOT`, and it is why a disclosure is not a
    /// thing to log.
    pub fn salt(&self) -> &str {
        &self.salt
    }

    /// The claim this discloses, or `None` for an array element, which has no
    /// name.
    pub fn claim_name(&self) -> Option<&str> {
        self.claim_name.as_deref()
    }

    /// Whether this discloses an array element (clause 4.2.2) and not an object
    /// property (clause 4.2.1).
    pub fn is_array_element(&self) -> bool {
        self.claim_name.is_none()
    }

    /// The value this discloses.
    pub fn claim_value(&self) -> &Value {
        &self.claim_value
    }

    /// The digest that stands in for this claim: base64url(SHA-256(ASCII(encoded))),
    /// per RFC 9901 clause 4.2.3. It goes in an `_sd` array for a property (clause
    /// 4.2.4.1) and in a `{"...": "<digest>"}` placeholder for an array element
    /// (clause 4.2.4.2).
    pub fn digest(&self) -> String {
        digest_of(&self.encoded)
    }
}

/// The digest of an already-encoded disclosure string.
///
/// Free function because a verifier hashes strings it has not parsed yet: an
/// unreadable disclosure must still be digested to be reported as unmatched,
/// rather than silently dropped.
pub fn digest_of(encoded: &str) -> String {
    b64().encode(Sha256::digest(encoded.as_bytes()))
}
