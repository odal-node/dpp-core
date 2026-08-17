//! eIDAS qualified electronic seal value objects — what a seal *is* once
//! produced, and what an adapter reports about its own capabilities.
//!
//! These are **persisted domain values**, not ports: [`SealedEnvelope`] is
//! serialised onto `Passport::seal` and travels on the wire. The trait that
//! produces one is the extension seam and lives in [`crate::ports::seal`],
//! which also carries the regulatory basis for what "qualified" requires.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─── Types ───────────────────────────────────────────────────────────────────

/// Which eIDAS sealing model the request should use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SealMode {
    /// Platform holds its own qualified seal; operators use delegated access.
    ///
    /// **The legal basis for this mode is not established by the registry
    /// rules.** Verified against the OJ text of IR 2026/1778: Art. 19(4) permits
    /// a verified economic operator to authorise a third party to perform
    /// *"registration actions in the registry"* on its behalf, provided that
    /// third party follows the verification process in accordance with Art. 5.
    /// That is delegated **registration**, and it says nothing about who may
    /// hold or use a qualified electronic seal.
    ///
    /// Art. 19(5) is likewise about data rather than seals: each verified
    /// economic operator *"shall be responsible for the data it submits to the
    /// Commission as manager of the registry and shall be considered as the
    /// controller of the data it submits"*.
    ///
    /// So delegation of registration is settled and delegation of sealing is
    /// not. Whether one party may hold a qualified seal covering content another
    /// party authored is a question under eIDAS and the applicable delegated
    /// act, not one these articles answer — and the mechanics moving would not
    /// move the responsibility either way.
    ProviderSeal,
    /// Operator holds and manages their own qualified seal.
    OperatorSeal,
}

/// AdES signature/seal format family.
///
/// JAdES is the primary format: JSON-native, built on JWS (RFC 7515), and
/// the natural fit for DPP payloads. The others are modelled for completeness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum SealFormat {
    /// JSON Advanced Electronic Signatures (ETSI TS 119 182-1) — primary path.
    Jades,
    /// PDF Advanced Electronic Signatures.
    Pades,
    /// CMS Advanced Electronic Signatures (binary/CMS).
    Cades,
    /// XML Advanced Electronic Signatures.
    Xades,
}

/// How much validation material the seal carries with it.
///
/// The AdES baseline levels, named as the CSC API names them
/// (`conformance_level`). They are cumulative: each adds to the one before.
///
/// # Why this is on the request and not left to the adapter
///
/// **A `BaselineB` seal on a ten-year passport stops verifying when its signing
/// certificate expires.** The level decides whether a verifier years from now
/// can still establish that the seal was valid when it was made, and ESPR
/// retention outlives certificate lifetimes comfortably. The seal is bought once
/// and the document it covers is retention-locked, so this cannot be corrected
/// afterwards by re-sealing — the same irreversibility that makes the refusal
/// rule on [`SealCapabilities::can_produce`] worth having.
///
/// Leaving it implicit meant a caller could not ask for long-term validity and
/// could not tell they had not got it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum SealConformanceLevel {
    /// `AdES-B-B` — the signature alone. No timestamp, no validation material.
    ///
    /// Verifiable only while the signing certificate is valid and its status is
    /// still resolvable. Adequate for a short-lived attestation and not for a
    /// passport.
    BaselineB,
    /// `AdES-B-T` — adds a trusted timestamp, so the signing *time* is
    /// established independently of the signer's clock.
    BaselineT,
    /// `AdES-B-LT` — adds the certificates and revocation data a verifier needs,
    /// so the seal remains verifiable after the signing certificate expires.
    ///
    /// **The first level that survives certificate expiry**, and therefore the
    /// first that suits a retention-locked document.
    BaselineLt,
    /// `AdES-B-LTA` — adds archival timestamps, extending validity past the
    /// cryptographic lifetime of the algorithms themselves.
    BaselineLta,
}

impl SealConformanceLevel {
    /// Every level this build models. Same reasoning as [`SealFormat::ALL`].
    pub const ALL: &'static [Self] = &[
        Self::BaselineB,
        Self::BaselineT,
        Self::BaselineLt,
        Self::BaselineLta,
    ];

    /// Whether a seal at this level stays verifiable after its signing
    /// certificate expires.
    ///
    /// The property that actually matters for a retention-locked passport, named
    /// so a caller can ask for it without having to know which letters mean what.
    #[must_use]
    pub const fn survives_certificate_expiry(self) -> bool {
        matches!(self, Self::BaselineLt | Self::BaselineLta)
    }
}

/// Where the seal sits relative to the bytes it covers.
///
/// # Why the port models both rather than choosing
///
/// A qualified seal **can** be a JWS signature: JAdES (ETSI TS 119 182-1) is an
/// AdES format built on RFC 7515, and its scope covers qualified electronic
/// seals explicitly. So "the seal is the payload's signature" and "the seal
/// wraps a digest of the payload" are both available at the standards layer, and
/// JAdES itself spans them — its `sigD` header identifies a detached payload
/// precisely because RFC 7515 has no native way to.
///
/// What decides which is available is **the provider's format menu, not the
/// law**. Those move independently, so a port that picked one would encode a
/// supplier's product decision as an architectural one. It is a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum SealEnvelope {
    /// The seal is returned alongside the data, covering a digest of it.
    /// Two artefacts: the payload signs itself, the seal attests it separately.
    Detached,
    /// The signed data is carried inside the seal structure. One artefact.
    Enveloping,
    /// The seal is embedded into the document it covers, which stays readable
    /// in its own format.
    Attached,
}

impl SealEnvelope {
    /// Every packaging this build models. Same reasoning as [`SealFormat::ALL`].
    pub const ALL: &'static [Self] = &[Self::Detached, Self::Enveloping, Self::Attached];
}

/// A CSC-style reference to a QTSP-held credential. Never contains key material.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealCredentialRef {
    /// Identifier of the Qualified Trust Service Provider.
    pub qtsp_id: String,
    /// Credential identifier within the QTSP (CSC `credentialID`).
    pub credential_id: String,
}

/// Input to a seal operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealRequest {
    /// SHA-256 hex digest of the payload to seal.
    pub payload_hash: String,
    /// Which eIDAS sealing model to use.
    pub mode: SealMode,
    /// Reference to the QTSP-held credential (not key material).
    pub key_ref: SealCredentialRef,
    /// Desired AdES envelope format. JAdES is recommended.
    pub sig_format: SealFormat,
    /// How much validation material the seal must carry.
    ///
    /// Defaults to [`SealConformanceLevel::BaselineLt`] on the wire — the
    /// weakest level that survives certificate expiry. A default that did not
    /// would quietly hand a retention-locked passport a seal with a shelf life.
    #[serde(default = "default_conformance_level")]
    pub conformance_level: SealConformanceLevel,
    /// Where the seal sits relative to the bytes it covers.
    ///
    /// Defaults to [`SealEnvelope::Detached`], which is what a request built
    /// from a payload hash means: the caller already holds the bytes and wants
    /// an attestation over them.
    #[serde(default = "default_envelope")]
    pub envelope: SealEnvelope,
}

fn default_conformance_level() -> SealConformanceLevel {
    SealConformanceLevel::BaselineLt
}

fn default_envelope() -> SealEnvelope {
    SealEnvelope::Detached
}

/// A completed qualified seal envelope returned by the QTSP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealedEnvelope {
    /// AdES format of this seal value.
    pub format: SealFormat,
    /// Base64-encoded seal value as returned by the QTSP.
    pub seal_value: String,
    /// Optional reference to the signing certificate chain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signing_cert_ref: Option<String>,
    /// Timestamp when the seal was created.
    pub sealed_at: DateTime<Utc>,
    /// True when this envelope was produced by `GhostSeal` and has no legal validity.
    pub placeholder: bool,
}

impl SealFormat {
    /// Every format this build models, for exhaustive iteration.
    ///
    /// `SealFormat` is `#[non_exhaustive]`, so a consumer outside this crate
    /// cannot enumerate it — and the conformance kit has to, in order to ask an
    /// adapter for a format it does **not** advertise. A format added later is
    /// deliberately not covered until it is added here on purpose.
    pub const ALL: &'static [Self] = &[Self::Jades, Self::Pades, Self::Cades, Self::Xades];
}

impl SealMode {
    /// Every mode this build models. Same reasoning as [`SealFormat::ALL`].
    pub const ALL: &'static [Self] = &[Self::ProviderSeal, Self::OperatorSeal];
}

/// Which seal profiles an adapter supports.
///
/// Four axes, mirroring what the CSC API's `credentials/info` reports back —
/// `signature_formats`, `conformance_levels`, and the envelope properties each
/// format admits. Capability discovery is in that protocol already; this is the
/// same idea at the port.
#[derive(Debug, Clone)]
pub struct SealCapabilities {
    pub supported_formats: Vec<SealFormat>,
    pub supported_modes: Vec<SealMode>,
    /// Baseline levels this adapter can produce. An adapter offering only
    /// [`SealConformanceLevel::BaselineB`] cannot seal a retention-locked
    /// document in a way that outlives its own certificate — a procurement
    /// problem, and one a caller can now see rather than discover.
    pub supported_levels: Vec<SealConformanceLevel>,
    /// Packagings this adapter can produce.
    pub supported_envelopes: Vec<SealEnvelope>,
}

impl SealCapabilities {
    /// Whether these capabilities cover what `req` asks for.
    ///
    /// Defined once, here, so every adapter answers the question the same way.
    /// An adapter that rolled its own check would be free to disagree with the
    /// capabilities it advertises, which is the disagreement the check exists to
    /// make impossible.
    ///
    /// **Every** axis must match, and each carries meaning no other can stand in
    /// for. A provider that produces the right format under the wrong
    /// certificate holder has not produced what was asked for — the mode decides
    /// *whose* attestation the seal is. One that produces the right format at a
    /// lower baseline level has delivered a seal with a shorter life than the
    /// document it covers. Neither is a serialisation detail.
    pub fn can_produce(&self, req: &SealRequest) -> bool {
        self.supported_formats.contains(&req.sig_format)
            && self.supported_modes.contains(&req.mode)
            && self.supported_levels.contains(&req.conformance_level)
            && self.supported_envelopes.contains(&req.envelope)
    }

    /// Whether this adapter can produce any seal that outlives its signing
    /// certificate.
    ///
    /// A node whose only sealing provider answers `false` here can issue
    /// passports whose seals stop verifying long before the retention period
    /// ends. Surfaced as a question an operator can ask at boot, rather than one
    /// discovered years later by a verifier.
    #[must_use]
    pub fn can_outlive_certificate_expiry(&self) -> bool {
        self.supported_levels
            .iter()
            .any(|l| l.survives_certificate_expiry())
    }
}

/// The outcome of validating a seal.
///
/// Three-valued, because AdES validation is. The names follow the status
/// indications in **ETSI EN 319 102-1**, the standard that specifies how an AdES
/// signature is validated, so a verdict produced here maps onto one produced by
/// any conformant validator without a translation step that could lose its
/// meaning.
///
/// The middle value is the reason this is not a boolean.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SealIndication {
    /// The seal passed the validation that was performed.
    ///
    /// What that is worth depends entirely on [`SealChecks`] — a pass over a
    /// signature check alone is a far smaller claim than a pass over a full
    /// validation, and the two must never be read as the same statement.
    TotalPassed,

    /// The format is wrong, or the signature value failed verification.
    ///
    /// A definite negative: this seal does not attest what it claims to.
    TotalFailed(String),

    /// Validation did not fail, but there was not enough information to decide.
    ///
    /// The ordinary answer whenever material has to be fetched — revocation data
    /// unreachable, a timestamp not yet corroborated, a trust anchor
    /// unresolvable at the moment of asking. It means *ask again later*, not
    /// *reject this passport*, and collapsing it either way is wrong: to failed,
    /// and a sound passport is reported non-compliant; to passed, and a check
    /// that never completed is claimed as one that did.
    Indeterminate(String),
}

/// How far validation got — what a [`SealIndication`] is actually founded on.
///
/// Without this, `TotalPassed` from two very different checks is one value. They
/// are not one claim: proving a signature is consistent with the certificate it
/// carries says nothing about whether that certificate was qualified, current,
/// or issued by anyone trustworthy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SealChecks {
    /// Nothing was checked — the verdict describes the envelope, not a
    /// validation. What a placeholder yields.
    None,
    /// The signature was checked against the certificate carried inside the
    /// seal, and nothing else: no certificate path, no revocation, no timestamp,
    /// no Trusted List.
    SignatureOnly,
    /// A full AdES validation: certificate path to a trust anchor, revocation
    /// status and timestamp, as well as the signature.
    FullValidation,
}

/// Result of verifying a `SealedEnvelope`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealVerification {
    /// What the validation concluded.
    pub indication: SealIndication,
    /// What was actually checked to reach it.
    pub checks: SealChecks,
    /// True if this was a ghost/placeholder seal (always unverified in production).
    pub placeholder: bool,
}

impl SealVerification {
    /// A pass, founded on `checks`.
    ///
    /// Takes the checks rather than defaulting them, because "what was
    /// verified" is the whole content of a pass. There is no sensible default:
    /// guessing high overstates the claim and guessing low understates it.
    #[must_use]
    pub fn passed(checks: SealChecks) -> Self {
        Self {
            indication: SealIndication::TotalPassed,
            checks,
            placeholder: false,
        }
    }

    /// A definite negative — the seal does not attest what it claims to.
    #[must_use]
    pub fn failed(checks: SealChecks, reason: impl Into<String>) -> Self {
        Self {
            indication: SealIndication::TotalFailed(reason.into()),
            checks,
            placeholder: false,
        }
    }

    /// Validation did not fail, but there was not enough information to decide.
    ///
    /// The ordinary answer whenever material has to be fetched. Reach for this
    /// rather than [`Self::failed`] when the check did not complete: a sound
    /// passport reported non-compliant because a revocation endpoint was
    /// unreachable is a defect, not caution.
    #[must_use]
    pub fn indeterminate(checks: SealChecks, reason: impl Into<String>) -> Self {
        Self {
            indication: SealIndication::Indeterminate(reason.into()),
            checks,
            placeholder: false,
        }
    }

    /// The verdict for a placeholder envelope: nothing checked, nothing decided.
    ///
    /// Indeterminate rather than failed, and the distinction matters. A
    /// placeholder is not a seal that failed validation; it is a seal no
    /// validation was attempted on. Reporting it as failed would put a definite
    /// negative on a passport whose seal nobody has looked at.
    #[must_use]
    pub fn placeholder(reason: impl Into<String>) -> Self {
        Self {
            indication: SealIndication::Indeterminate(reason.into()),
            checks: SealChecks::None,
            placeholder: true,
        }
    }

    /// Whether this is a pass that a relying party may rest a compliance claim on.
    ///
    /// A named method because the mistake it prevents is the easy one to make:
    /// reading `TotalPassed` alone as "this is a valid qualified seal", when the
    /// check behind it may have been a bare signature comparison against a
    /// self-signed certificate. Requiring both parts at every call site would
    /// work exactly as well right up until one site forgot.
    #[must_use]
    pub fn is_qualified_pass(&self) -> bool {
        !self.placeholder
            && self.checks == SealChecks::FullValidation
            && self.indication == SealIndication::TotalPassed
    }

    /// Whether the verdict is internally consistent.
    ///
    /// One combination is incoherent: `TotalPassed` founded on
    /// [`SealChecks::None`] — a pass over nothing checked. That is not a
    /// stricter or looser claim than the others, it is a claim with no referent,
    /// and it is precisely the shape of the worst defect this port could ship: a
    /// verifier reporting a seal it never examined as good.
    ///
    /// `TotalFailed` with `None` is coherent, and deliberately so — an envelope
    /// can be rejected on its format before any validation is attempted.
    ///
    /// The fields are public, so this cannot be an unrepresentable state without
    /// a breaking redesign of a persisted value object. It is instead checkable,
    /// and [`crate::ports::seal::conformance`] checks it for every verdict an
    /// adapter produces.
    #[must_use]
    pub fn is_coherent(&self) -> bool {
        !(self.indication == SealIndication::TotalPassed && self.checks == SealChecks::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_format_serde_round_trips() {
        for fmt in [
            SealFormat::Jades,
            SealFormat::Pades,
            SealFormat::Cades,
            SealFormat::Xades,
        ] {
            let json = serde_json::to_string(&fmt).unwrap();
            let back: SealFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(fmt, back);
        }
    }

    /// A pass is not a pass is not a pass.
    ///
    /// The whole reason the verdict is not a boolean: `TotalPassed` over a bare
    /// signature check and `TotalPassed` over a full AdES validation are
    /// different claims, and only the second is one a compliance decision may
    /// rest on. If these ever collapse, a self-signed development seal satisfies
    /// the same test a qualified one does.
    #[test]
    fn a_signature_check_is_not_a_qualified_pass() {
        let signature_only = SealVerification {
            indication: SealIndication::TotalPassed,
            checks: SealChecks::SignatureOnly,
            placeholder: false,
        };
        assert!(
            !signature_only.is_qualified_pass(),
            "a signature check says nothing about the certificate behind it"
        );

        let full = SealVerification {
            checks: SealChecks::FullValidation,
            ..signature_only.clone()
        };
        assert!(full.is_qualified_pass());

        // And a placeholder never passes, however it is labelled.
        let placeholder = SealVerification {
            placeholder: true,
            ..full.clone()
        };
        assert!(!placeholder.is_qualified_pass());
    }

    /// Indeterminate is neither pass nor fail, and must not be read as either.
    #[test]
    fn indeterminate_is_not_a_pass() {
        let unresolved = SealVerification {
            indication: SealIndication::Indeterminate("revocation data unreachable".into()),
            checks: SealChecks::FullValidation,
            placeholder: false,
        };
        assert!(!unresolved.is_qualified_pass());
        assert_ne!(unresolved.indication, SealIndication::TotalPassed);
    }

    #[test]
    fn seal_mode_serde_round_trips() {
        for mode in [SealMode::ProviderSeal, SealMode::OperatorSeal] {
            let json = serde_json::to_string(&mode).unwrap();
            let back: SealMode = serde_json::from_str(&json).unwrap();
            assert_eq!(mode, back);
        }
    }
}
