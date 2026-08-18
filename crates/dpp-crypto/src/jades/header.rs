//! The JAdES protected header — and which parameters ETSI TS 119 182-1
//! actually requires at the B-B level.
//!
//! Every rule below is transcribed from V1.2.1 (2024-07), not recalled. The
//! clause numbers are here so a reader can check rather than trust.

use base64::Engine;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

/// Why a header could not be built.
///
/// Deliberately one variant. The other rule worth breaking — omitting the
/// signing-certificate reference entirely — is unrepresentable: [`JadesHeader`]
/// takes a [`CertificateRef`] rather than an `Option<CertificateRef>`, so the
/// clause 5.1.7 requirement is carried by the type instead of by a check that
/// could be forgotten. An error variant for it would be an error nobody can
/// produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JadesError {
    /// `x5c` was supplied with no certificates in it.
    ///
    /// A chain is the certificate reference, so an empty one leaves the
    /// signature with none — the case clause 5.1.7 forbids, reached through the
    /// one door the type cannot close.
    EmptyCertificateChain,
}

impl std::fmt::Display for JadesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyCertificateChain => write!(
                f,
                "x5c was supplied with no certificates, leaving the signature without                  the certificate reference TS 119 182-1 clause 5.1.7 requires"
            ),
        }
    }
}

impl std::error::Error for JadesError {}

/// How the signing certificate is identified inside the signed header.
///
/// Clause 5.1.7: *"A JAdES signature shall have at least one of the following
/// header parameters in its JWS Protected Header: `x5t#S256`, `x5c`, `sigX5ts`,
/// or `x5t#o`."* Requiring this type rather than an `Option` of it is how that
/// "at least one" is enforced — the two forms modelled here are the two in
/// common use.
///
/// Both are signed, so both bind the signature to a certificate. They differ in
/// what a verifier has to already possess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CertificateRef {
    /// `x5t#S256` — the base64url-encoded SHA-256 digest of the DER-encoded
    /// signing certificate (RFC 7515 clause 4.1.8).
    ///
    /// Compact, and it commits to exactly one certificate — but a verifier that
    /// does not already hold that certificate cannot obtain it from the
    /// signature.
    Thumbprint(String),
    /// `x5c` — the certificate chain, each entry a **base64** (not base64url,
    /// and not padded differently) DER certificate, signing certificate first
    /// (RFC 7515 clause 4.1.6).
    ///
    /// Larger, and self-contained: a verifier gets the certificate with the
    /// signature. For a passport that has to stay verifiable for years, this is
    /// usually the one worth the bytes — the alternative assumes a certificate
    /// is still retrievable from somewhere when it matters.
    ///
    /// **On its own this is not enough for a baseline signature.** See
    /// [`Self::ChainWithThumbprint`].
    Chain(Vec<String>),
    /// `x5c` **and** `x5t#S256` together — the chain, plus a digest reference
    /// to the signing certificate. **The form to use.**
    ///
    /// # Why both, when clause 5.1.7 says one is enough
    ///
    /// Because 5.1.7 and Table 1 are answering different questions, and only
    /// reading both makes the difference visible.
    ///
    /// Clause 5.1.7 states the minimum for *a JAdES signature*: at least one of
    /// `x5t#S256`, `x5c`, `sigX5ts` or `x5t#o`. Table 1 states what a *baseline*
    /// signature requires, and there the service "signing a reference of the
    /// signing certificate" has cardinality **1** with only the three digest
    /// forms as its options — `x5c` is a separate row entirely.
    ///
    /// So a signature carrying `x5c` alone satisfies 5.1.7 and is still not
    /// B-B. The European Commission's DSS says so directly: it reported such a
    /// signature as form JAdES at level **`JSON-NOT-ETSI`**, warning that *"the
    /// signed attribute: 'signing-certificate' is absent"*.
    ///
    /// That was found by an outside implementation, not by reading — which is
    /// what an oracle is for. Clause 5.1.7 NOTE 1 explicitly contemplates the
    /// simultaneous presence of these parameters, so carrying both is
    /// conformant and gets the self-containment of the chain as well.
    ChainWithThumbprint {
        /// Base64 DER certificates, signing certificate first (RFC 7515 4.1.6).
        chain: Vec<String>,
        /// Base64url SHA-256 of the signing certificate's DER (RFC 7515 4.1.8).
        thumbprint: String,
    },
}

impl CertificateRef {
    /// Compute an `x5t#S256` thumbprint from a DER-encoded certificate.
    ///
    /// The digest is over the DER bytes, base64url-encoded without padding, per
    /// RFC 7515 clause 4.1.8.
    #[must_use]
    pub fn thumbprint_of_der(der: &[u8]) -> Self {
        let digest = Sha256::digest(der);
        Self::Thumbprint(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest))
    }

    /// Both parameters, derived from the DER of the chain.
    ///
    /// The signing certificate is the first entry, per RFC 7515 clause 4.1.6,
    /// and the thumbprint is taken over it.
    ///
    /// # Errors
    ///
    /// [`JadesError::EmptyCertificateChain`] for an empty chain — there would be
    /// no signing certificate to digest.
    pub fn chain_of_der(chain: &[Vec<u8>]) -> Result<Self, JadesError> {
        let signing = chain.first().ok_or(JadesError::EmptyCertificateChain)?;
        let Self::Thumbprint(thumbprint) = Self::thumbprint_of_der(signing) else {
            unreachable!("thumbprint_of_der always yields a Thumbprint");
        };
        Ok(Self::ChainWithThumbprint {
            chain: chain
                .iter()
                .map(|der| base64::engine::general_purpose::STANDARD.encode(der))
                .collect(),
            thumbprint,
        })
    }

    fn insert_into(&self, map: &mut Map<String, Value>) -> Result<(), JadesError> {
        let chain_value =
            |chain: &Vec<String>| Value::Array(chain.iter().cloned().map(Value::String).collect());
        match self {
            Self::Thumbprint(t) => {
                map.insert("x5t#S256".to_owned(), Value::String(t.clone()));
                Ok(())
            }
            Self::Chain(chain) | Self::ChainWithThumbprint { chain, .. } if chain.is_empty() => {
                Err(JadesError::EmptyCertificateChain)
            }
            Self::Chain(chain) => {
                map.insert("x5c".to_owned(), chain_value(chain));
                Ok(())
            }
            Self::ChainWithThumbprint { chain, thumbprint } => {
                map.insert("x5c".to_owned(), chain_value(chain));
                map.insert("x5t#S256".to_owned(), Value::String(thumbprint.clone()));
                Ok(())
            }
        }
    }
}

/// A JAdES-B-B protected header for an **attached** payload.
///
/// # What the standard requires, and what it deliberately does not
///
/// From Table 1 (Requirements for JAdES-B-B … signatures) and the clauses it
/// references:
///
/// | Parameter | B-B | Why |
/// |---|---|---|
/// | `alg` | **shall be present** | cardinality 1 |
/// | claimed signing time | **shall be provided** | via `iat` — see below |
/// | certificate reference | **cardinality 1** | one of `x5t#S256`/`x5c`/`sigX5ts`/`x5t#o`, clause 5.1.7 |
/// | `cty` | conditioned | content type of the payload |
/// | `crit` | conditioned | **only** required when `sigD` is present |
/// | `sigD` | may be present | *"shall not appear in JAdES signatures whose JWS Payload is attached"* |
///
/// **`iat`, not `sigT`.** Clause 5.1.11: *"Starting at 2025-07-15T00:00:00Z,
/// this header parameter shall be incorporated in new JAdES signatures."* That
/// date has passed, so `iat` is mandatory and `sigT` is the legacy spelling.
/// Its value is an integer number of seconds and *"shall not contain fractions
/// of seconds"*.
///
/// **No `crit`, and that is correct.** V1.1.1 required every JAdES signed
/// header parameter to be named in `crit`. V1.2.1 **suppressed** that rule —
/// clause 5.1.9 NOTE 1 says it was *"qualifying it as problematic because it
/// could not be properly managed by plane JWS processing applications"*, and
/// that with the change *"any JAdES signature that does not incorporate the
/// `sigD` header parameter can be (partly) processed by a plane JWS processing
/// application"*.
///
/// Since an attached payload forbids `sigD`, and `sigD` is the only thing that
/// forces `crit`, a JAdES-B-B signature built here carries no `crit` and stays
/// readable by any RFC 7515 library. That is a property worth keeping: it means
/// the same artefact serves a verifier that understands AdES and one that only
/// understands JWS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JadesHeader {
    /// JOSE `alg`, e.g. `"EdDSA"` or `"RS256"`. Whatever the signing key uses.
    pub alg: String,
    /// Claimed signing time as whole seconds since the Unix epoch.
    pub iat: i64,
    /// How the signing certificate is identified.
    pub certificate: CertificateRef,
    /// `cty` — the payload's content type, when it needs stating.
    pub content_type: Option<String>,
}

impl JadesHeader {
    /// Build a header, claiming *now* as the signing time.
    ///
    /// Infallible: `timestamp()` yields whole seconds, which is exactly what
    /// clause 5.1.11 asks for, and a `CertificateRef` is required by the
    /// signature rather than checked for.
    #[must_use]
    pub fn now(alg: impl Into<String>, certificate: CertificateRef) -> Self {
        Self {
            alg: alg.into(),
            iat: chrono::Utc::now().timestamp(),
            certificate,
            content_type: None,
        }
    }

    /// Set the `cty` content type.
    #[must_use]
    pub fn with_content_type(mut self, cty: impl Into<String>) -> Self {
        self.content_type = Some(cty.into());
        self
    }

    /// Serialise to the exact JSON bytes that will be base64url-encoded and
    /// signed.
    ///
    /// # On key order
    ///
    /// These bytes are covered by the signature, so order matters: two
    /// serialisations differing only in key order are two different signing
    /// inputs. Without `serde_json`'s `preserve_order` feature a `Map` is a
    /// `BTreeMap`, so the output is **sorted**, not insertion-ordered.
    ///
    /// Sorted is fine — JWS places no constraint on header key order, and
    /// determinism is what actually matters. But the guarantee this module
    /// relies on is stronger and does not depend on the feature at all:
    /// [`super::prepare`] encodes the header **once** and
    /// [`super::PreparedJades`] retains the encoded segment, so what is
    /// assembled is byte-identical to what was signed even if a future feature
    /// unification changed the ordering underneath. Nothing recomputes a header
    /// and compares.
    ///
    /// # Errors
    ///
    /// Propagates [`CertificateRef`] validation.
    pub fn to_json_bytes(&self) -> Result<Vec<u8>, JadesError> {
        let mut map = Map::new();
        map.insert("alg".to_owned(), Value::String(self.alg.clone()));
        if let Some(cty) = &self.content_type {
            map.insert("cty".to_owned(), Value::String(cty.clone()));
        }
        self.certificate.insert_into(&mut map)?;
        map.insert("iat".to_owned(), Value::Number(self.iat.into()));

        // Infallible: every value inserted above is a string, an array of
        // strings, or an integer.
        Ok(serde_json::to_vec(&Value::Object(map)).expect("header is plain JSON"))
    }
}
