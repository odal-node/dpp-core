//! [`SealVerification`] — a verification outcome and how much was actually checked.

use super::indication::SealIndication;

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
    /// A complete **AdES** validation: certificate path to a trust anchor,
    /// revocation status and timestamp, as well as the signature.
    ///
    /// What an ordinary AdES validation library gives you, and the level at
    /// which the seal is cryptographically sound and its certificate is
    /// current.
    ///
    /// **It does not establish that the seal is qualified**, and the gap is not
    /// a matter of degree — see [`Self::QualifiedValidation`]. A certificate can
    /// chain to a trust anchor, be unrevoked and carry a valid timestamp while
    /// being an ordinary organisational certificate from a CA that is not a
    /// QTSP.
    AdesValidation,
    /// An AdES validation **plus** the two legs that make a seal qualified.
    ///
    /// Named for the legal outcome rather than the mechanism, because the
    /// mechanism is what keeps getting mistaken for sufficient.
    ///
    /// # What this requires beyond [`Self::AdesValidation`]
    ///
    /// Regulation (EU) No 910/2014 as amended. **Art. 40** applies **Art. 32**
    /// *mutatis mutandis* to seals, so Art. 32(1) is the definition — read with
    /// Annex III in place of Annex I and Art. 36 in place of Art. 26. The
    /// conditions an AdES validation does not reach:
    ///
    /// - **Art. 32(1)(a)–(b)** — the certificate was, at the time of sealing, a
    ///   **qualified** certificate complying with Annex III, **issued by a
    ///   QTSP**. That is a Trusted List question (**Art. 22**), and a path to
    ///   *some* trust anchor does not answer it.
    /// - **Art. 32(1)(f)** — the seal was created by a **qualified electronic
    ///   seal creation device**. **Annex III(j)** makes this machine-detectable:
    ///   the certificate carries an indication, in a form suitable for automated
    ///   processing, that the creation data resides in such a device.
    ///
    /// # There is a third level in the law, and this pair does not model it
    ///
    /// Worth stating, because the boundary is easy to put in the wrong place.
    /// eIDAS 2 inserted **Art. 40a** — validation of *advanced* seals based on a
    /// **qualified certificate** — separately from Art. 40. Its conditions
    /// mirror Art. 32(1)(a)–(b) but **omit the creation-device leg**, because an
    /// advanced seal is not required to use a qualified device.
    ///
    /// So the ladder is really: a generic AdES validation, which consults no
    /// Trusted List at all → Art. 32a/40a, which does → Art. 32/40, which adds
    /// the device leg. Commission Implementing Regulation (EU) 2025/1945 lays
    /// the last two out in exactly that shape, giving Art. 32(3)/40 its own
    /// Annex I and Art. 32a(3)/40a its own Annex II — and **both** annexes
    /// reference ETSI TS 119 612 *Trusted Lists*, which is what places the
    /// trusted-list leg below the middle rung rather than at the top.
    ///
    /// [`Self::AdesValidation`] is the bottom rung and this variant is the top.
    /// The middle one is deliberately not modelled: nothing produces it, and a
    /// variant no adapter reaches is a distinction callers have to handle
    /// without ever being able to test it. Add it when an adapter genuinely
    /// implements Art. 40a and stops there — at which point it wants the name
    /// `QualifiedCertificateValidation` and the reasoning above.
    QualifiedValidation,
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
    ///
    /// # It requires [`SealChecks::QualifiedValidation`], not merely a complete AdES one
    ///
    /// This method used to accept what is now [`SealChecks::AdesValidation`] —
    /// certificate path, revocation, timestamp, signature. That is everything an
    /// AdES validation library returns, and it is **below the bar for
    /// qualified**: it establishes neither that the certificate was a qualified
    /// certificate issued by a QTSP (Art. 32(1)(a)–(b) via Art. 40, a Trusted
    /// List question) nor that a qualified creation device was used
    /// (Art. 32(1)(f), detectable via Annex III(j)).
    ///
    /// Nothing was wrong at the time, because no adapter produced that verdict.
    /// The hazard was ahead: the first real validator integration satisfies an
    /// AdES-complete check by construction, and this method would then have
    /// started returning `true` for seals nobody had shown to be qualified,
    /// with nothing at the call site looking amiss.
    ///
    /// No kit can check that an adapter claiming `QualifiedValidation` really
    /// consulted a Trusted List — that is a claim about work done elsewhere, and
    /// [`crate::ports::seal::conformance`] only reaches the verdicts it can
    /// falsify (a pass over nothing checked, a placeholder that passes). What
    /// the split buys is that the claim now has to be *made* deliberately
    /// instead of arriving as a by-product of an ordinary AdES validation.
    #[must_use]
    pub fn is_qualified_pass(&self) -> bool {
        !self.placeholder
            && self.checks == SealChecks::QualifiedValidation
            && self.indication == SealIndication::TotalPassed
    }

    /// Whether the seal is cryptographically sound and its certificate current,
    /// without any claim that it is qualified.
    ///
    /// The honest reading of an [`SealChecks::AdesValidation`] pass, and the
    /// answer to a genuinely different question: *did this seal verify?* rather
    /// than *may a compliance claim rest on it?* An adapter that reaches only
    /// this far is not defective — it is most AdES tooling — and a caller that
    /// wants the weaker statement should be able to ask for it by name rather
    /// than by comparing enum variants and getting the boundary wrong.
    ///
    /// True for [`SealChecks::QualifiedValidation`] as well, since that is
    /// strictly more.
    #[must_use]
    pub fn is_ades_pass(&self) -> bool {
        !self.placeholder
            && matches!(
                self.checks,
                SealChecks::AdesValidation | SealChecks::QualifiedValidation
            )
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
