//! VC-JOSE-COSE: any W3C Verifiable Credential or Verifiable Presentation, secured
//! as a compact JWS, as an SD-JWT, or with COSE.
//!
//! Implements **Securing Verifiable Credentials using JOSE and COSE**, the W3C
//! Recommendation of 13 October 2026 (`REC-vc-jose-cose-20261013`), whose normative
//! text is that of the 15 May 2025 edition: a credential is a
//! `vc+jwt` (§3.1.1) and a presentation is a `vp+jwt` (§3.1.2), each a compact JWS
//! (RFC 7515) whose payload is the credential or presentation itself; a `vc+sd-jwt`
//! (§3.2.1) and a `vp+sd-jwt` (§3.2.2), each an SD-JWT (RFC 9901) whose JWT Claims Set is
//! the credential or presentation; or a `vc+cose` (§3.3.1) and a `vp+cose` (§3.3.2), each
//! a `COSE_Sign1` (RFC 9052) whose payload is the credential or presentation itself.
//! [`crate::sd_jwt_vc`] is a different format, the IETF SD-JWT VC profile.
//!
//! **No conformance is claimed.** What was read and what each part is checked
//! against is recorded in the standards register. A claim of conformance waits for
//! the W3C test suite's results, all of them.
//!
//! # How this differs from the access credential
//!
//! [`crate::credential::jwt`] signs one typed document, a [`DppAccessCredential`],
//! and verifies it through an issuer's `did:web` document. This module signs and
//! verifies **any** credential or presentation, given as JSON, under a key the
//! caller supplies. It resolves nothing and performs no I/O: a token's `kid` is a
//! hint (§4.1.1), and choosing the key is the caller's, the way
//! [`sd_jwt_vc::verify`](crate::sd_jwt_vc::verify) leaves it.
//!
//! [`DppAccessCredential`]: crate::credential::DppAccessCredential
//!
//! # Signing
//!
//! [`sign_credential`] and [`sign_presentation`] take the document, a
//! [`KeyEntry`](dpp_crypto::keystore::KeyEntry), the `kid`, and the extension terms
//! the caller means to use. The header carries `alg`, `kid`, `typ` and `cty`, and
//! the payload is the document unchanged in RFC 8785 canonical form. A key from
//! [`KeyEntry::from_private_jwk`](dpp_crypto::keystore::KeyEntry::from_private_jwk)
//! signs as well as one a key store generated, under EdDSA, ES256, ES384 or ES512.
//!
//! **Undeclared members are refused at signing.** A top-level member must be a term
//! of the VC Data Model 2.0 base vocabulary ([`CREDENTIAL_TERMS`] or
//! [`PRESENTATION_TERMS`]), a JWT registered claim name of RFC 7519 §4.1, or named
//! in `extension_terms`. `vc` and `vp` are refused even when named (§1.1.2.1).
//! Verification ignores members it does not understand, as §3.1.3 and §5.4 require:
//! the discipline is an issuer's, not a verifier's.
//!
//! Signing also refuses what verification would refuse for the same document: an `iss`
//! that is not the issuer, and an `exp` or `nbf` that is not a JSON number. It refuses an
//! `iat` that is not a number too (RFC 7519 §4.1.6), though verification does not read `iat`.
//! No time is compared at signing.
//!
//! # Verifying a credential
//!
//! [`verify_credential`] takes the issuer's [`PublicKey`](dpp_crypto::jws::PublicKey)
//! and refuses at the first failed check, in this order: compact form, protected
//! header (readable, no `crit`, no `b64` of `false`, `alg` is the key's, so `alg:
//! none` is refused), signature, `typ` and `cty`, payload and its `@context`, `vc`
//! and `vp`, time, the audience, the issuer binding of §4.1.2, then the credential's
//! `type`, `issuer` and `credentialSubject`. Every refusal is a variant of [`JoseError`].
//!
//! **The `kid` is a hint, and no verifier holds it to a form.** §4.2 asks for an absolute
//! URL as the `kid` when there is no `iss` and the issuer is a URL, but it opens "When using
//! controlled identifier documents with this specification", and §5 assumes key discovery "has
//! succeeded prior to beginning the verification process". A verifier here is given its key,
//! so it has no document to hold the `kid` against, and a token whose `kid` is a bare
//! thumbprint verifies, in every form. `iss`, when present, must still be the issuer. The
//! signers keep the rule: a `kid` that is not an absolute URL is refused
//! ([`JoseError::KidNotAbsolute`]) when there is no `iss` and the issuer, or the holder of a
//! presentation, is a URL, so the tokens this crate writes stay strict.
//!
//! **Time is explicit.** A [`VerifyOptions`] carries `now`, seconds since the epoch, and an
//! optional leeway for `exp` and `nbf`, capped at [`MAX_LEEWAY_SECONDS`]. Leeway defaults to
//! zero. `sd_jwt_vc::verify` takes the same options for the same JWT time claims. With no `now`,
//! JOSE and COSE do not read `exp` or `nbf`. `iat` is never checked: §3.1.3 says it describes
//! the signature, not the credential. A credential's own `validFrom` and `validUntil` are
//! checked separately by `verify_credential_claims` and do not use this leeway.
//!
//! **The audience is explicit too.** The same [`VerifyOptions`] carries the audience the
//! caller identifies as. RFC 7519 §4.1.3 says a JWT "MUST be rejected" when the principal
//! processing it "does not identify itself with a value in the "aud" claim when this claim
//! is present". So a token whose `aud`, a string or an array of strings, does not name the
//! audience is refused; a caller that gives none is refused any token that carries `aud`; an
//! `aud` of another shape is refused. The same rule runs on the processed payload of an
//! SD-JWT and on every envelope a presentation holds.
//!
//! # Verifying a presentation
//!
//! Verifying a presentation and verifying the credentials it envelops are two
//! operations. VC-JOSE-COSE §5.1 defines verifying a presentation secured with JOSE as
//! validating its own JWT (RFC 7519), §5.3 defines it for COSE as the `COSE_Sign1`
//! process on the presentation itself, and §5 says key discovery is assumed to have
//! succeeded before each document is verified. So [`verify_presentation`],
//! [`verify_presentation_sd_jwt`] and [`verify_presentation_cose`] check the presentation's
//! own token completely, then judge each envelope **as an envelope**: its `type`, its
//! `@context`, and that its `id` is a `data:` URL with a media type (§3.1.2, §3.2.2, §3.3.2;
//! VC DM 2.0 §4.13). They do not read what is inside it, a compact JWS, an SD-JWT or a
//! `COSE_Sign1`. They return a [`VerifiedPresentation`] whose envelopes are **not**
//! verified. Each credential is verified afterwards, under the key the caller has for its
//! issuer: one at a time with [`UnverifiedEnvelope::verify`], or all of them with
//! [`VerifiedPresentation::verify_credentials`], which fails if any one fails. That is where
//! the content is read, and where content that cannot be read is refused, wrapped in
//! [`JoseError::CredentialRefused`] with the variant that says why. Only a `vc+jwt`, a
//! `vc+sd-jwt` and a `vc+cose` can be verified, so an envelope of another format makes
//! `verify_credentials` fail with `UnsupportedEnvelopeFormat`. §5.4 asks for further
//! validation once verification succeeds; the checks this crate does not make are listed
//! below.
//!
//! # The SD-JWT form
//!
//! [`sign_credential_sd_jwt`] and [`sign_presentation_sd_jwt`] take what the `vc+jwt`
//! signers take, and the claims to conceal as [`ClaimPath`](dpp_crypto::sd_jwt::ClaimPath)s,
//! and return the [`SdJwt`](dpp_crypto::sd_jwt::SdJwt) with every Disclosure.
//! [`verify_credential_sd_jwt`] and [`verify_presentation_sd_jwt`] take its combined
//! format, `<JWT>~<Disclosure>~...~`, the key, the [`VerifyOptions`] and a
//! [`KeyBindingRequirement`], for which there is no default.
//!
//! - **The checks run on the processed payload.** VC-JOSE-COSE §5.2 says to convert the
//!   SD-JWT payload back into the JWT Claims Set. So `@context`, `type`, `issuer`, `holder`,
//!   `credentialSubject`, `iss` and the time claims are read from the claims
//!   once the Disclosures are applied, with the rules of the `vc+jwt` form, and the header
//!   rules are read from the Issuer-signed JWT. A claim that was concealed and not disclosed
//!   is absent, and a required one that is absent is refused.
//! - **What may be concealed.** §3.2.1 says "implementers SHOULD ensure that properties
//!   necessary for the validation and verification of a credential are NOT selectively
//!   disclosable", and names `@context`, `type`, `credentialStatus`, `credentialSchema` and
//!   `relatedResource`. A signer here refuses a path that starts at one of them, and at
//!   `iss`, `aud`, `exp`, `nbf` and `cnf`, which RFC 9901 clause 9.7 forbids an Issuer to
//!   make selectively disclosable. A verifier does not refuse a token for the way a claim
//!   travelled: one of those that was disclosed is checked as it is in the clear, since
//!   the checks read the processed payload (clause 7.1 step 6), so a disclosed `exp` in the
//!   past is refused as expired when the caller gives a `now`, as one in the clear is.
//!   One that was withheld cannot be seen. A verifier does not refuse a token for
//!   concealing `type` or the others in §3.2.1, which are a SHOULD: the suite's own
//!   presentation conceals `type`.
//! - **Paths.** A path names an object member by its name, and an array element by its
//!   position in the document as the issuer wrote it. `null` stands for every element,
//!   and each must hold the rest of the path. A path that selects nothing is refused, so a
//!   claim the issuer meant to conceal is not signed in the clear. A path to a claim and a
//!   path inside it conceal both, the inner Disclosure inside the outer one.
//! - **`typ`** is required and names `vc+sd-jwt` or `vp+sd-jwt`, and `cty`, when present,
//!   names `vc` or `vp`, as for the `vc+jwt` form, though §3.2 says SHOULD for each.
//! - **A member named `_sd`, `_sd_alg` or `...`** is refused at signing: a reader would take
//!   it for the mechanism and remove it (RFC 9901 clauses 4.1.1 and 4.2.1).
//! - **Presentations** are verified as in the `vp+jwt` form: the presentation's own SD-JWT,
//!   then each envelope as an envelope, then nothing inside it. A `vc+sd-jwt` envelope
//!   (`data:application/vc+sd-jwt,<sd-jwt>`) is verified by
//!   [`UnverifiedEnvelope::verify`] and by [`VerifiedPresentation::verify_credentials`],
//!   under the key the caller supplies, as strictly as a credential given on its own.
//! - **Key Binding is the verifier's decision, and has no default.** RFC 9901 §9.5 says
//!   "Verifiers MUST decide whether Key Binding is required for a particular use case before
//!   verifying a credential", and §7.3 step 1 that the decision "MUST NOT be based on whether
//!   or not a Key Binding JWT is provided by the Holder". So every SD-JWT verifier, and the
//!   envelope path, takes a [`KeyBindingRequirement`]. [`NotRequired`] reads a token as an
//!   SD-JWT and ignores a Key Binding JWT it carries (§7.3 steps 3 and 4).
//!   [`Required`] applies the checks of §7.3 step 5 with a
//!   [`KeyBindingPolicy`](dpp_crypto::sd_jwt::KeyBindingPolicy), the audience, the nonce
//!   and the window, to the Key Binding JWT, under the Holder key in the processed payload's
//!   `cnf`, and refuses a token with no Key Binding JWT or no `cnf`. The signers take an
//!   optional holder key and write it as `cnf` (VC-JOSE-COSE §4.1.3, RFC 7800 §3.2);
//!   the holder binds a presentation with
//!   [`SdJwt::present_with_key_binding`](dpp_crypto::sd_jwt::SdJwt::present_with_key_binding).
//!
//! [`NotRequired`]: KeyBindingRequirement::NotRequired
//! [`Required`]: KeyBindingRequirement::Required
//!
//! # The COSE form
//!
//! [`sign_credential_cose`] and [`sign_presentation_cose`] take what the `vc+jwt` signers
//! take and return the tagged `COSE_Sign1`. [`verify_credential_cose`] and
//! [`verify_presentation_cose`] take its bytes, the key and the [`VerifyOptions`].
//!
//! - **The payload is the document.** §3.3.1: "The unsecured verifiable credential is the
//!   unencoded COSE_Sign1 payload". It is the UTF-8 JSON of the document, in RFC 8785 canonical
//!   form when signed, and it is not a claims set: nothing is written into it, and no JWT claim
//!   is read out of it. A member named `exp` or `iss` is an undeclared term at signing like
//!   any other, [`CREDENTIAL_TERMS`] and `extension_terms` are all that is allowed, and `vc` and
//!   `vp` are refused at signing as undeclared terms and not by §1.1.2.1, which is stated for
//!   a JWT Claims Set. The same VC Data Model 2.0 checks as the other two forms apply.
//! - **Headers.** The signer writes the protected `alg` (the key's), `kid` (the UTF-8 bytes of
//!   the identifier given), `typ` (16) `application/vc+cose` or `application/vp+cose`, and the
//!   content type (3) `application/vc` or `application/vp`. The verifier holds `typ` and the
//!   content type to the policy the `vc+jwt` form applies to `typ` and `cty`: `typ` required, each
//!   value naming the media type with or without the `application/` prefix, though §3.3.1 says
//!   SHOULD for both.
//! - **CWT claims** (§3.3.3, RFC 9597, RFC 8392) are read from the CWT Claims header parameter of
//!   the protected header. The signers write none. `iss` must be the issuer (§4.1.2), `exp` and
//!   `nbf` are checked under the `now` of [`VerifyOptions`], and `aud` follows the audience rule
//!   above, each as the same JWT claim is in the `vc+jwt` form. The parameter is refused in the
//!   unprotected header, where nothing protects it.
//! - **Envelopes.** A `vc+cose` credential in a presentation is
//!   `data:application/vc+cose;base64,<base64>` (§3.3.1). The base64 is read with the WHATWG
//!   Infra Standard's "forgiving-base64 decode" and not RFC 2045's, which pads: the W3C suite's
//!   own envelope has no padding. Where Key Binding is required a `vc+cose` envelope is refused,
//!   as a `vc+jwt` envelope is, since it cannot carry a Key Binding JWT.
//!
//! # What is deliberately absent
//!
//! - **JSON-LD expansion.** `@context` is checked against the VC Data Model 2.0 context
//!   set (§4.3), and nothing is fetched or expanded: this crate does no network access.
//! - **Key discovery.** §4.2 leaves how a verifier finds a key to the verifier, and
//!   this crate does not check that a `kid` names a method the issuer controls, or that it
//!   is an absolute URL.
//! - **`credentialStatus` and `credentialSchema` checks** (§5.4 SHOULD), and the
//!   credential's validity period.
//! - **`crit`.** Any `crit` is refused, which is stricter than RFC 7515 §4.1.11,
//!   since no critical extension is understood.

mod absolute_url;
mod cose_sign;
mod cose_verify;
mod data_url;
mod disclose;
mod envelope_shape;
pub mod error;
mod options;
mod presentation;
mod properties;
mod sd_jwt_sign;
mod sd_jwt_verify;
mod sign;
mod unverified_envelope;
mod verified_presentation;
mod verify;
mod vocabulary;

#[cfg(test)]
mod tests;

pub use cose_sign::{sign_credential_cose, sign_presentation_cose};
pub use cose_verify::{verify_credential_cose, verify_presentation_cose};
pub use error::JoseError;
pub use options::{KeyBindingRequirement, MAX_LEEWAY_SECONDS, VerifyOptions};
pub use presentation::verify_presentation;
pub use sd_jwt_sign::{sign_credential_sd_jwt, sign_presentation_sd_jwt};
pub use sd_jwt_verify::{verify_credential_sd_jwt, verify_presentation_sd_jwt};
pub use sign::{sign_credential, sign_presentation};
pub use unverified_envelope::UnverifiedEnvelope;
pub use verified_presentation::VerifiedPresentation;
pub(crate) use verify::check_audience;
pub use verify::verify_credential;
pub use vocabulary::{CREDENTIAL_TERMS, PRESENTATION_TERMS};
