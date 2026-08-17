//! A conformance kit any [`SealPort`] implementation can be held to.
//!
//! # Why this exists
//!
//! [`SealPort::seal`] states a contract — *an implementation must refuse what it
//! cannot produce* — and until this module that contract was a doc comment.
//! Doc comments are not enforcement, and the gap showed: the only real adapter
//! in the project builds its envelope with a hardcoded format regardless of what
//! the caller asked for, and never consults
//! [`SealCapabilities::can_produce`]. A contract with one implementor that does
//! not honour it is weaker than no contract, because it reads as a guarantee.
//!
//! The kit lives in `dpp-core` rather than beside any adapter deliberately. The
//! contract belongs to the port, so the checks belong to the port; an adapter
//! that wrote its own would be free to test the behaviour it happens to have.
//!
//! # What it does not do
//!
//! It cannot tell you a seal is *qualified*. That is a statement about a
//! certificate, a creation device and a QTSP — see [`crate::ports::seal`] for
//! the three-part conjunction eIDAS Art. 3(27) requires — and none of it is
//! observable from this side of the trait. This kit checks that an adapter's
//! behaviour agrees with its own advertisement, and that no verdict it produces
//! is internally incoherent. Both are necessary and neither is sufficient.
//!
//! # Usage
//!
//! ```no_run
//! # use dpp_domain::ports::seal::{SealPort, conformance};
//! # async fn check(adapter: &impl SealPort) {
//! let report = conformance::check_seal_port(adapter).await;
//! assert!(report.is_conformant(), "{report}");
//! # }
//! ```
//!
//! A failing adapter is a defect. A **note** is not: an adapter that cannot
//! verify reports that here rather than failing, because "produces seals,
//! cannot check them" is a real and currently-occupied position — it is simply
//! one an operator should know they are in.

use std::fmt;

use super::{
    SealCapabilities, SealConformanceLevel, SealCredentialRef, SealEnvelope, SealFormat,
    SealIndication, SealMode, SealPort, SealRequest, SealVerification,
};

/// A conformance finding: something an adapter got wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceFailure {
    /// Short identifier for the rule broken, e.g. `"seal.substituted_format"`.
    pub rule: &'static str,
    /// What happened, with the inputs that produced it.
    pub detail: String,
}

/// The outcome of running the kit against one adapter.
///
/// Separates **failures** (contract violations) from **notes** (things an
/// operator should know that are not violations). Collapsing the two would
/// force a choice between failing an adapter for not implementing verification
/// — which the trait permits — and staying silent about it, which is how
/// "this node cannot check its own seals" stops being visible.
#[derive(Debug, Clone, Default)]
pub struct ConformanceReport {
    /// Contract violations. Any entry means the adapter is non-conformant.
    pub failures: Vec<ConformanceFailure>,
    /// Observations worth surfacing that are not violations.
    pub notes: Vec<String>,
    /// How many (format, mode) pairs were exercised.
    pub combinations_checked: usize,
}

impl ConformanceReport {
    /// Whether the adapter honoured every rule the kit checks.
    #[must_use]
    pub fn is_conformant(&self) -> bool {
        self.failures.is_empty()
    }

    fn fail(&mut self, rule: &'static str, detail: impl Into<String>) {
        self.failures.push(ConformanceFailure {
            rule,
            detail: detail.into(),
        });
    }
}

impl fmt::Display for ConformanceReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "seal port conformance: {} failure(s), {} note(s), {} combination(s) checked",
            self.failures.len(),
            self.notes.len(),
            self.combinations_checked
        )?;
        for failure in &self.failures {
            writeln!(f, "  FAIL [{}] {}", failure.rule, failure.detail)?;
        }
        for note in &self.notes {
            writeln!(f, "  note: {note}")?;
        }
        Ok(())
    }
}

/// A request that asks for exactly `(format, mode)`.
///
/// The payload hash is a well-formed SHA-256 hex digest so an adapter that
/// validates its input shape accepts it — the kit is testing the capability
/// contract, and a rejection for the wrong reason would look like conformance.
/// A request naming all four axes explicitly.
fn profiled_request(
    format: SealFormat,
    mode: SealMode,
    conformance_level: SealConformanceLevel,
    envelope: SealEnvelope,
) -> SealRequest {
    SealRequest {
        payload_hash: "ab".repeat(32),
        mode,
        key_ref: SealCredentialRef {
            qtsp_id: "conformance".into(),
            credential_id: "conformance".into(),
        },
        sig_format: format,
        conformance_level,
        envelope,
    }
}

/// Run the full kit against `adapter`.
///
/// Rules checked:
///
/// 1. **`seal.refused_advertised`** — every advertised `(format, mode)` pair is
///    actually produced. An adapter that advertises what it will not do sends
///    callers down a path that fails at runtime.
/// 2. **`seal.substituted_format`** — the envelope comes back in the format that
///    was asked for. Silent substitution is the defect this contract exists to
///    prevent: sealing is bought and the document is retention-locked, so it
///    cannot be undone once noticed.
/// 3. **`seal.accepted_unadvertised`** — a `(format, mode)` outside the
///    advertisement is refused, not fulfilled. Both axes count: the mode decides
///    *whose* attestation the seal is.
/// 4. **`verify.incoherent_verdict`** — no verdict is `TotalPassed` founded on
///    `SealChecks::None`, a pass over nothing checked.
/// 5. **`verify.placeholder_passed`** — a placeholder envelope never satisfies
///    [`SealVerification::is_qualified_pass`].
///
/// The adapter is expected to be a test or development instance: this calls
/// `seal` once per advertised pair and would spend real money against a live
/// QTSP.
pub async fn check_seal_port<P: SealPort + ?Sized>(adapter: &P) -> ConformanceReport {
    let mut report = ConformanceReport::default();
    let capabilities = adapter.capabilities();

    if capabilities.supported_formats.is_empty() || capabilities.supported_modes.is_empty() {
        report.notes.push(
            "adapter advertises no formats or no modes; only the refusal rules were exercised"
                .to_owned(),
        );
    }

    if !capabilities.can_outlive_certificate_expiry() {
        report.notes.push(
            "no advertised conformance level survives certificate expiry (B-LT or higher) — \
             seals from this adapter stop verifying when the signing certificate does, which \
             is inside the retention period of any passport it seals"
                .to_owned(),
        );
    }

    check_advertised(adapter, &capabilities, &mut report).await;
    check_unadvertised(adapter, &capabilities, &mut report).await;

    report
}

/// Rules 1, 2, 4 and 5 — everything reachable through an advertised pair.
async fn check_advertised<P: SealPort + ?Sized>(
    adapter: &P,
    capabilities: &SealCapabilities,
    report: &mut ConformanceReport,
) {
    let mut verify_unsupported = false;

    for format in &capabilities.supported_formats {
        for mode in &capabilities.supported_modes {
            report.combinations_checked = report.combinations_checked.saturating_add(1);
            // The first advertised level and envelope, so every advertised
            // format/mode pair is still exercised exactly once. The remaining
            // level and envelope combinations are covered by the refusal sweep
            // below, which is where a mismatch actually shows.
            let level = capabilities
                .supported_levels
                .first()
                .copied()
                .unwrap_or(SealConformanceLevel::BaselineLt);
            let packaging = capabilities
                .supported_envelopes
                .first()
                .copied()
                .unwrap_or(SealEnvelope::Detached);
            let req = profiled_request(format.clone(), mode.clone(), level, packaging);

            let envelope = match adapter.seal(req).await {
                Ok(envelope) => envelope,
                Err(e) => {
                    report.fail(
                        "seal.refused_advertised",
                        format!("advertised {format:?}/{mode:?} but refused it: {e}"),
                    );
                    continue;
                }
            };

            if envelope.format != *format {
                report.fail(
                    "seal.substituted_format",
                    format!(
                        "asked for {format:?}, received {:?} — a substituted attestation, \
                         not the one the caller chose",
                        envelope.format
                    ),
                );
            }

            match adapter.verify(&envelope).await {
                Ok(verification) => audit_verdict(&verification, format, mode, report),
                // Permitted: the trait does not require an adapter to verify.
                // Recorded once, because "can seal, cannot check" is a position
                // an operator should know they are in.
                Err(_) => verify_unsupported = true,
            }
        }
    }

    if verify_unsupported {
        report.notes.push(
            "verify() is unsupported for at least one advertised profile — this adapter can \
             produce seals it cannot check, including its own"
                .to_owned(),
        );
    }
}

/// Rules 4 and 5 against one verdict.
fn audit_verdict(
    verification: &SealVerification,
    format: &SealFormat,
    mode: &SealMode,
    report: &mut ConformanceReport,
) {
    if !verification.is_coherent() {
        report.fail(
            "verify.incoherent_verdict",
            format!(
                "{format:?}/{mode:?} returned {:?} founded on {:?} — a pass over nothing checked",
                verification.indication, verification.checks
            ),
        );
    }
    if verification.placeholder && verification.is_qualified_pass() {
        report.fail(
            "verify.placeholder_passed",
            format!("{format:?}/{mode:?} reported a placeholder envelope as a qualified pass"),
        );
    }
    if let SealIndication::TotalFailed(reason) | SealIndication::Indeterminate(reason) =
        &verification.indication
        && reason.trim().is_empty()
    {
        report.notes.push(format!(
            "{format:?}/{mode:?} returned a non-pass verdict with an empty reason; an operator \
             cannot act on it"
        ));
    }
}

/// Rule 3 — every `(format, mode)` pair outside the advertisement is refused.
async fn check_unadvertised<P: SealPort + ?Sized>(
    adapter: &P,
    capabilities: &SealCapabilities,
    report: &mut ConformanceReport,
) {
    for format in SealFormat::ALL {
        for mode in SealMode::ALL {
            for level in SealConformanceLevel::ALL {
                for packaging in SealEnvelope::ALL {
                    let req = profiled_request(format.clone(), mode.clone(), *level, *packaging);
                    if capabilities.can_produce(&req) {
                        continue;
                    }
                    report.combinations_checked = report.combinations_checked.saturating_add(1);
                    if let Ok(produced) = adapter.seal(req).await {
                        report.fail(
                            "seal.accepted_unadvertised",
                            format!(
                                "does not advertise {format:?}/{mode:?}/{level:?}/{packaging:?} \
                                 but produced a {:?} envelope for it",
                                produced.format
                            ),
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::ghosts::GhostSeal;
    use crate::ports::seal::{SealChecks, SealedEnvelope};
    use async_trait::async_trait;
    use chrono::Utc;

    /// The reference implementation passes its own contract.
    #[tokio::test]
    async fn the_ghost_is_conformant() {
        let report = check_seal_port(&GhostSeal).await;
        assert!(report.is_conformant(), "{report}");
        assert!(
            report.combinations_checked >= SealFormat::ALL.len(),
            "every format must be exercised in one direction or the other: {report}"
        );
    }

    /// An adapter that substitutes a format is caught.
    ///
    /// This is the engine's current shape, reproduced here so the kit is known
    /// to detect it rather than assumed to.
    struct SubstitutesFormat;

    #[async_trait]
    impl SealPort for SubstitutesFormat {
        async fn seal(
            &self,
            _req: SealRequest,
        ) -> Result<SealedEnvelope, crate::domain::error::DppError> {
            Ok(SealedEnvelope {
                // Whatever was asked for, this is what you get.
                format: SealFormat::Cades,
                seal_value: "synthetic".into(),
                signing_cert_ref: None,
                sealed_at: Utc::now(),
                placeholder: false,
            })
        }
        async fn verify(
            &self,
            _env: &SealedEnvelope,
        ) -> Result<SealVerification, crate::domain::error::DppError> {
            Ok(SealVerification::passed(SealChecks::FullValidation))
        }
        fn capabilities(&self) -> SealCapabilities {
            SealCapabilities {
                supported_formats: vec![SealFormat::Jades, SealFormat::Cades],
                supported_modes: vec![SealMode::ProviderSeal],
                supported_levels: vec![SealConformanceLevel::BaselineLt],
                supported_envelopes: vec![SealEnvelope::Detached],
            }
        }
    }

    #[tokio::test]
    async fn a_substituted_format_is_caught() {
        let report = check_seal_port(&SubstitutesFormat).await;
        assert!(!report.is_conformant(), "{report}");
        assert!(
            report
                .failures
                .iter()
                .any(|f| f.rule == "seal.substituted_format"),
            "{report}"
        );
        // It also fulfils modes it never advertised.
        assert!(
            report
                .failures
                .iter()
                .any(|f| f.rule == "seal.accepted_unadvertised"),
            "{report}"
        );
    }

    /// A verifier claiming a pass over nothing checked is caught.
    struct PassesOverNothing;

    #[async_trait]
    impl SealPort for PassesOverNothing {
        async fn seal(
            &self,
            req: SealRequest,
        ) -> Result<SealedEnvelope, crate::domain::error::DppError> {
            Ok(SealedEnvelope {
                format: req.sig_format,
                seal_value: "synthetic".into(),
                signing_cert_ref: None,
                sealed_at: Utc::now(),
                placeholder: false,
            })
        }
        async fn verify(
            &self,
            _env: &SealedEnvelope,
        ) -> Result<SealVerification, crate::domain::error::DppError> {
            Ok(SealVerification {
                indication: SealIndication::TotalPassed,
                checks: SealChecks::None,
                placeholder: false,
            })
        }
        fn capabilities(&self) -> SealCapabilities {
            SealCapabilities {
                supported_formats: vec![SealFormat::Jades],
                supported_modes: vec![SealMode::ProviderSeal],
                supported_levels: vec![SealConformanceLevel::BaselineLt],
                supported_envelopes: vec![SealEnvelope::Detached],
            }
        }
    }

    #[tokio::test]
    async fn a_pass_over_nothing_checked_is_caught() {
        let report = check_seal_port(&PassesOverNothing).await;
        assert!(!report.is_conformant(), "{report}");
        assert!(
            report
                .failures
                .iter()
                .any(|f| f.rule == "verify.incoherent_verdict"),
            "{report}"
        );
    }

    /// An adapter that can only produce short-lived seals is noted.
    ///
    /// `B-B` carries no validation material, so the seal stops verifying when
    /// its signing certificate expires — comfortably inside the retention period
    /// of any passport it covers. That is a procurement fact rather than a
    /// contract violation, so it is a note; the point is that it is *visible*
    /// before a decade of passports depend on it.
    struct ShortLivedOnly;

    #[async_trait]
    impl SealPort for ShortLivedOnly {
        async fn seal(
            &self,
            req: SealRequest,
        ) -> Result<SealedEnvelope, crate::domain::error::DppError> {
            if !self.capabilities().can_produce(&req) {
                return Err(crate::domain::error::DppError::Validation(
                    crate::domain::field_error::ValidationErrors::message("profile not advertised"),
                ));
            }
            Ok(SealedEnvelope {
                format: req.sig_format,
                seal_value: "synthetic".into(),
                signing_cert_ref: None,
                sealed_at: Utc::now(),
                placeholder: false,
            })
        }
        async fn verify(
            &self,
            _env: &SealedEnvelope,
        ) -> Result<SealVerification, crate::domain::error::DppError> {
            Ok(SealVerification::passed(SealChecks::FullValidation))
        }
        fn capabilities(&self) -> SealCapabilities {
            SealCapabilities {
                supported_formats: vec![SealFormat::Cades],
                supported_modes: vec![SealMode::ProviderSeal],
                supported_levels: vec![SealConformanceLevel::BaselineB],
                supported_envelopes: vec![SealEnvelope::Detached],
            }
        }
    }

    #[tokio::test]
    async fn an_adapter_that_cannot_outlive_its_certificate_is_noted() {
        let report = check_seal_port(&ShortLivedOnly).await;
        assert!(
            report.is_conformant(),
            "offering only B-B is honest, not a contract breach: {report}"
        );
        assert!(
            report
                .notes
                .iter()
                .any(|n| n.contains("certificate expiry")),
            "but an operator must be told: {report}"
        );
    }

    /// Asking for long-term validity and being handed a bare signature is caught.
    ///
    /// The substitution this axis exists to prevent, and the one with the
    /// longest tail: a `B-B` seal where `B-LT` was requested looks identical
    /// until the certificate expires, years after the passport was locked.
    #[tokio::test]
    async fn a_downgraded_conformance_level_is_refused() {
        let caps = ShortLivedOnly.capabilities();
        let long_lived = profiled_request(
            SealFormat::Cades,
            SealMode::ProviderSeal,
            SealConformanceLevel::BaselineLt,
            SealEnvelope::Detached,
        );
        assert!(
            !caps.can_produce(&long_lived),
            "a B-B-only adapter must not claim it can produce B-LT"
        );
        assert!(!caps.can_outlive_certificate_expiry());
        assert!(
            ShortLivedOnly.seal(long_lived).await.is_err(),
            "and it must refuse rather than quietly hand back B-B"
        );
    }

    /// An adapter that cannot verify is noted, not failed.
    ///
    /// Otherwise conformant on purpose — it honours the refusal rule — so this
    /// test isolates the verification gap instead of also tripping over one.
    struct SealsButCannotVerify;

    #[async_trait]
    impl SealPort for SealsButCannotVerify {
        async fn seal(
            &self,
            req: SealRequest,
        ) -> Result<SealedEnvelope, crate::domain::error::DppError> {
            if !self.capabilities().can_produce(&req) {
                return Err(crate::domain::error::DppError::Validation(
                    crate::domain::field_error::ValidationErrors::message("profile not advertised"),
                ));
            }
            Ok(SealedEnvelope {
                format: req.sig_format,
                seal_value: "synthetic".into(),
                signing_cert_ref: None,
                sealed_at: Utc::now(),
                placeholder: false,
            })
        }
        async fn verify(
            &self,
            _env: &SealedEnvelope,
        ) -> Result<SealVerification, crate::domain::error::DppError> {
            Err(crate::domain::error::DppError::Validation(
                crate::domain::field_error::ValidationErrors::message("verification unsupported"),
            ))
        }
        fn capabilities(&self) -> SealCapabilities {
            SealCapabilities {
                supported_formats: vec![SealFormat::Cades],
                supported_modes: vec![SealMode::ProviderSeal],
                supported_levels: vec![SealConformanceLevel::BaselineLt],
                supported_envelopes: vec![SealEnvelope::Detached],
            }
        }
    }

    #[tokio::test]
    async fn an_adapter_that_cannot_verify_is_noted_not_failed() {
        let report = check_seal_port(&SealsButCannotVerify).await;
        assert!(
            report.is_conformant(),
            "not implementing verify is permitted by the trait: {report}"
        );
        assert!(
            report.notes.iter().any(|n| n.contains("cannot check")),
            "but it must be surfaced: {report}"
        );
    }
}
