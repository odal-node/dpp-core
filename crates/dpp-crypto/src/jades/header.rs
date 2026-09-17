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
/// Both variants are the same rule read twice: a header parameter that is
/// present and empty is not a present header parameter. *Omitting* the
/// signing-certificate reference entirely is unrepresentable — [`JadesHeader`]
/// takes a [`CertificateRef`] rather than an `Option<CertificateRef>`, so the
/// clause 5.1.7 requirement is carried by the type — but the variants' fields
/// are public, and an empty `Vec` or `String` is the one door the type cannot
/// close.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum JadesError {
    /// `x5c` was supplied with no certificates in it.
    ///
    /// A chain is the certificate reference, so an empty one leaves the
    /// signature with none — the case clause 5.1.7 forbids, reached through the
    /// one door the type cannot close.
    EmptyCertificateChain,
    /// `x5t#S256` was supplied as an empty string.
    ///
    /// Table 1 gives the digest reference cardinality 1, and a parameter
    /// serialised as `""` references no certificate — so the signature would
    /// carry the header without carrying the reference.
    EmptyThumbprint,
}

impl std::fmt::Display for JadesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyCertificateChain => write!(
                f,
                "x5c was supplied with no certificates, leaving the signature without                  the certificate reference TS 119 182-1 clause 5.1.7 requires"
            ),
            Self::EmptyThumbprint => write!(
                f,
                "x5t#S256 was supplied as an empty string, so the signature carries \
                 the parameter without the signing-certificate reference TS 119 182-1 \
                 Table 1 gives cardinality 1"
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
    ///
    /// # 🚨 Conformant to the standard, and outside the EU profile
    ///
    /// This satisfies TS 119 182-1 clause 5.1.7 and Table 1, so it is a valid
    /// JAdES baseline signature. It is **not** in the list a Member State public
    /// sector body is obliged to recognise.
    ///
    /// Commission Implementing Regulation (EU) 2026/248 (OJ L, 2026/248 of
    /// 3.2.2026), which repealed Commission Implementing Decision (EU) 2015/1506
    /// and lays down the formats of advanced electronic signatures and seals
    /// those bodies must recognise, lists JAdES in its **Annex I** with one
    /// adaptation: it replaces clause 5.1.8 so that the `x5c` header parameter
    /// **shall be present** in the JAdES signature, as a signed or unsigned
    /// header parameter. Plain TS 119 182-1 leaves `x5c` optional — clause 5.1.7
    /// asks for *at least one* of `x5t#S256`, `x5c`, `sigX5ts` or `x5t#o`. The
    /// EU adaptation adds `x5c` on top of that.
    ///
    /// So this variant carries no `x5c` and falls outside Annex I, while
    /// [`Chain`](Self::Chain) has `x5c` and fails Table 1, and
    /// [`ChainWithThumbprint`](Self::ChainWithThumbprint) satisfies both. Ask
    /// [`is_eu_recognised_profile`](Self::is_eu_recognised_profile) rather than
    /// matching on the variant.
    ///
    /// It is kept because it is a legitimate JAdES signature and this crate is
    /// not EU-only.
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
    /// Whether a signature carrying this reference is in the format a Member
    /// State public sector body is obliged to recognise.
    ///
    /// The property that actually matters, named so a caller does not have to
    /// know which JOSE header parameters mean what — the same shape as
    /// `SealConformanceLevel::survives_certificate_expiry` in `dpp-domain`.
    ///
    /// `true` requires **both** legs, and they come from different documents:
    ///
    /// - **`x5c` present** — Commission Implementing Regulation (EU) 2026/248,
    ///   Annex I, which replaces TS 119 182-1 clause 5.1.8 with a text reading
    ///   that the `x5c` header parameter *"shall be present in the JAdES
    ///   signature, either as a signed or unsigned header parameter"*. The
    ///   unadapted standard leaves it optional.
    /// - **a digest reference present** — TS 119 182-1 Table 1, where the
    ///   baseline service "signing a reference of the signing certificate" has
    ///   cardinality 1 and offers only the three digest forms. `x5c` is a
    ///   separate row and does not satisfy it.
    ///
    /// Only [`ChainWithThumbprint`](Self::ChainWithThumbprint) has both, which
    /// is why it is the form to use and why
    /// [`chain_of_der`](Self::chain_of_der) can only produce it.
    ///
    /// # Why this is a method and not a comment
    ///
    /// The requirement lives in a **Regulation**, not in the ETSI document this
    /// module is otherwise written against. Someone checking this code against
    /// the standard alone would find `Thumbprint` perfectly conformant and have
    /// no reason to look further. A named property is what survives that reader.
    ///
    /// It says nothing about whether the certificate is any good, whether the
    /// seal is qualified, or whether it validates — only whether the format is
    /// the one Annex I lists.
    ///
    /// # 🚨 Present means non-empty, and the variant alone did not say that
    ///
    /// This was `matches!(self, Self::ChainWithThumbprint { .. })` — the variant
    /// and nothing else. [`chain_of_der`](Self::chain_of_der) cannot build an
    /// empty one, but the variant's fields are public, so
    /// `ChainWithThumbprint { chain: vec![], thumbprint: String::new() }` is a
    /// value any caller can make — and it answered `true`.
    ///
    /// Both legs above are *presence* requirements: Annex I says `x5c` "shall be
    /// present", and Table 1 wants a digest reference with cardinality 1. An
    /// `x5c` that is present and empty carries no certificate, and an empty
    /// `x5t#S256` references nothing, so neither leg was met by the value this
    /// method called EU-recognised. A compliance caller could accept a header
    /// identifying no certificate at all, and nothing downstream would disagree
    /// until the header is serialised — which caught the empty chain
    /// and not the empty thumbprint.
    ///
    /// Still deliberately **not** checked here: whether `thumbprint` is actually
    /// the digest of `chain[0]`. That is a question about whether the reference
    /// is *correct*, not whether the format is present, and this method's scope
    /// is the second. A caller needing the first wants a verifier, not a
    /// predicate — but see the type's tests, which pin that this is a known
    /// boundary rather than an oversight.
    #[must_use]
    pub fn is_eu_recognised_profile(&self) -> bool {
        match self {
            Self::ChainWithThumbprint { chain, thumbprint } => {
                !chain.is_empty() && !thumbprint.is_empty()
            }
            _ => false,
        }
    }

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
            // The same rule for the other half. An `x5t#S256` serialised as `""`
            // is a header parameter that is present and references nothing, and
            // letting it through would leave this in step with
            // `is_eu_recognised_profile` in one direction only: the predicate
            // would say no while the serialiser wrote the header anyway.
            Self::ChainWithThumbprint { thumbprint, .. } if thumbprint.is_empty() => {
                Err(JadesError::EmptyThumbprint)
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
