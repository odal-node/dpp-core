//! Seal conformance levels and what each one admits.

use super::conformance::*;
use crate::ports::ghosts::GhostSeal;
use crate::ports::seal::SealPort;
use crate::seal::{
    SealCapabilities, SealChecks, SealConformanceLevel, SealEnvelope, SealFormat, SealIndication,
    SealMode, SealRequest, SealVerification, SealedEnvelope,
};
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
    async fn seal(&self, _req: SealRequest) -> Result<SealedEnvelope, crate::error::dpp::DppError> {
        Ok(SealedEnvelope {
            // Whatever was asked for, this is what you get.
            format: SealFormat::Cades,
            seal_value: "synthetic".into(),
            signing_cert_ref: None,
            // Unknown to this fake — it never reads the request. `None` keeps
            // rule 6 silent so the test isolates the defect it is named for.
            conformance_level: None,
            sealed_at: Utc::now(),
            placeholder: false,
        })
    }
    async fn verify(
        &self,
        _env: &SealedEnvelope,
    ) -> Result<SealVerification, crate::error::dpp::DppError> {
        Ok(SealVerification::passed(SealChecks::AdesValidation))
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
    async fn seal(&self, req: SealRequest) -> Result<SealedEnvelope, crate::error::dpp::DppError> {
        Ok(SealedEnvelope {
            format: req.sig_format,
            seal_value: "synthetic".into(),
            signing_cert_ref: None,
            conformance_level: Some(req.conformance_level),
            sealed_at: Utc::now(),
            placeholder: false,
        })
    }
    async fn verify(
        &self,
        _env: &SealedEnvelope,
    ) -> Result<SealVerification, crate::error::dpp::DppError> {
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
    async fn seal(&self, req: SealRequest) -> Result<SealedEnvelope, crate::error::dpp::DppError> {
        if !self.capabilities().can_produce(&req) {
            return Err(crate::error::dpp::DppError::Validation(
                crate::field_error::ValidationErrors::message("profile not advertised"),
            ));
        }
        Ok(SealedEnvelope {
            format: req.sig_format,
            seal_value: "synthetic".into(),
            signing_cert_ref: None,
            conformance_level: Some(req.conformance_level),
            sealed_at: Utc::now(),
            placeholder: false,
        })
    }
    async fn verify(
        &self,
        _env: &SealedEnvelope,
    ) -> Result<SealVerification, crate::error::dpp::DppError> {
        Ok(SealVerification::passed(SealChecks::AdesValidation))
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
    async fn seal(&self, req: SealRequest) -> Result<SealedEnvelope, crate::error::dpp::DppError> {
        if !self.capabilities().can_produce(&req) {
            return Err(crate::error::dpp::DppError::Validation(
                crate::field_error::ValidationErrors::message("profile not advertised"),
            ));
        }
        Ok(SealedEnvelope {
            format: req.sig_format,
            seal_value: "synthetic".into(),
            signing_cert_ref: None,
            // Records no level, and that is conformant — see
            // `an_absent_level_is_not_a_misrecording`. An adapter that cannot
            // tell what its provider produced says so rather than guessing.
            conformance_level: None,
            sealed_at: Utc::now(),
            placeholder: false,
        })
    }
    async fn verify(
        &self,
        _env: &SealedEnvelope,
    ) -> Result<SealVerification, crate::error::dpp::DppError> {
        Err(crate::error::dpp::DppError::Validation(
            crate::field_error::ValidationErrors::message("verification unsupported"),
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

/// Advertises PAdES, and only packagings PAdES does not define.
///
/// The advertisement reads as complete — a format, a mode, a level and a
/// packaging are all present — but no well-formed request can name this
/// format, because the packagings belong to other formats entirely.
struct FormatWithNoPackaging;

#[async_trait]
impl SealPort for FormatWithNoPackaging {
    async fn seal(&self, _req: SealRequest) -> Result<SealedEnvelope, crate::error::dpp::DppError> {
        Ok(SealedEnvelope {
            format: SealFormat::Pades,
            seal_value: "synthetic".into(),
            signing_cert_ref: None,
            // Unknown to this fake — it never reads the request. `None` keeps
            // rule 6 silent so the test isolates the defect it is named for.
            conformance_level: None,
            sealed_at: Utc::now(),
            placeholder: false,
        })
    }
    async fn verify(
        &self,
        _env: &SealedEnvelope,
    ) -> Result<SealVerification, crate::error::dpp::DppError> {
        Ok(SealVerification::passed(SealChecks::AdesValidation))
    }
    fn capabilities(&self) -> SealCapabilities {
        SealCapabilities {
            supported_formats: vec![SealFormat::Pades],
            supported_modes: vec![SealMode::ProviderSeal],
            supported_levels: vec![SealConformanceLevel::BaselineLt],
            // Both belong to other formats; PAdES defines neither.
            supported_envelopes: vec![SealEnvelope::Detached, SealEnvelope::Enveloping],
        }
    }
}

#[tokio::test]
async fn a_format_with_no_advertised_packaging_is_caught() {
    let report = check_seal_port(&FormatWithNoPackaging).await;
    assert!(!report.is_conformant(), "{report}");
    assert!(
        report
            .failures
            .iter()
            .any(|f| f.rule == "capabilities.format_without_envelope"),
        "an unrequestable format is a defect in the advertisement, not in a request: {report}"
    );
}

/// Honours the request, then records a different level on the envelope.
///
/// Otherwise conformant on purpose, so this isolates the misrecording rather
/// than also tripping the format or refusal rules. The shape is not
/// hypothetical: an adapter that derives the level from its own configuration
/// rather than from the request it was handed produces exactly this, and the
/// derivation looks reasonable at every step.
struct MisrecordsLevel;

#[async_trait]
impl SealPort for MisrecordsLevel {
    async fn seal(&self, req: SealRequest) -> Result<SealedEnvelope, crate::error::dpp::DppError> {
        if !self.capabilities().can_produce(&req) {
            return Err(crate::error::dpp::DppError::Validation(
                crate::field_error::ValidationErrors::message("profile not advertised"),
            ));
        }
        Ok(SealedEnvelope {
            format: req.sig_format,
            seal_value: "synthetic".into(),
            signing_cert_ref: None,
            // Asked for B-LT, told the record it was B-B.
            conformance_level: Some(SealConformanceLevel::BaselineB),
            sealed_at: Utc::now(),
            placeholder: false,
        })
    }
    async fn verify(
        &self,
        _env: &SealedEnvelope,
    ) -> Result<SealVerification, crate::error::dpp::DppError> {
        Ok(SealVerification::passed(SealChecks::AdesValidation))
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

/// An envelope that misrecords its level is caught.
///
/// The failure this rule exists for is silent and permanent. The seal itself may
/// be perfectly good B-LT; what is wrong is the record beside it, and the record
/// is what an auditor reads. Because the passport is retention-locked, a wrong
/// level cannot be corrected once written — so it has to be refused when it is
/// produced, which is here.
#[tokio::test]
async fn an_envelope_that_misrecords_its_level_is_caught() {
    let report = check_seal_port(&MisrecordsLevel).await;
    assert!(!report.is_conformant(), "{report}");
    assert!(
        report
            .failures
            .iter()
            .any(|f| f.rule == "seal.misrecorded_level"),
        "a level that disagrees with the request is a substitution on the axis that \
         cannot be corrected later: {report}"
    );
}

/// Records the first advertised level faithfully and the second wrongly.
///
/// The shape the kit used to miss entirely. It probed only
/// `supported_levels.first()`, and the refusal sweep covers the levels an
/// adapter does *not* advertise — which is the wrong half, because a request
/// outside the advertisement is refused and produces no envelope to misrecord.
struct MisrecordsOnlyTheSecondLevel;

#[async_trait]
impl SealPort for MisrecordsOnlyTheSecondLevel {
    async fn seal(&self, req: SealRequest) -> Result<SealedEnvelope, crate::error::dpp::DppError> {
        if !self.capabilities().can_produce(&req) {
            return Err(crate::error::dpp::DppError::Validation(
                crate::field_error::ValidationErrors::message("profile not advertised"),
            ));
        }
        Ok(SealedEnvelope {
            format: req.sig_format,
            seal_value: "synthetic".into(),
            signing_cert_ref: None,
            conformance_level: Some(match req.conformance_level {
                // Honest about the first advertised level.
                SealConformanceLevel::BaselineT => SealConformanceLevel::BaselineT,
                // …and wrong about every other, in the dangerous direction.
                _ => SealConformanceLevel::BaselineB,
            }),
            sealed_at: Utc::now(),
            placeholder: false,
        })
    }
    async fn verify(
        &self,
        _env: &SealedEnvelope,
    ) -> Result<SealVerification, crate::error::dpp::DppError> {
        Ok(SealVerification::passed(SealChecks::AdesValidation))
    }
    fn capabilities(&self) -> SealCapabilities {
        SealCapabilities {
            supported_formats: vec![SealFormat::Cades],
            supported_modes: vec![SealMode::ProviderSeal],
            supported_levels: vec![
                SealConformanceLevel::BaselineT,
                SealConformanceLevel::BaselineLt,
            ],
            supported_envelopes: vec![SealEnvelope::Detached],
        }
    }
}

/// Every advertised level is probed, not just the first.
///
/// The one that matters most is the one furthest down the list: `B-LT` is where
/// a passport's seal starts surviving its signing certificate, so an adapter
/// that quietly records `B-B` there is misdescribing the only property that
/// cannot be corrected after the passport is locked.
#[tokio::test]
async fn a_misrecorded_level_is_caught_on_a_level_that_is_not_the_first() {
    let report = check_seal_port(&MisrecordsOnlyTheSecondLevel).await;
    assert!(!report.is_conformant(), "{report}");
    assert!(
        report
            .failures
            .iter()
            .any(|f| f.rule == "seal.misrecorded_level"),
        "a level advertised but never asked for is a level never checked: {report}"
    );
    assert!(
        report.combinations_checked >= 2,
        "both advertised levels must actually be requested: {report}"
    );
}

/// An adapter that records no level at all is conformant.
///
/// Deliberately permitted. `GhostSeal` echoes the request, but an adapter that
/// genuinely does not know what its provider produced has one honest answer, and
/// it is `None` — the same standing as an absent `signingCertRef`. Failing it
/// would push adapters into guessing, which is the defect the rule above exists
/// to catch.
#[tokio::test]
async fn an_absent_level_is_not_a_misrecording() {
    let report = check_seal_port(&SealsButCannotVerify).await;
    assert!(
        !report
            .failures
            .iter()
            .any(|f| f.rule == "seal.misrecorded_level"),
        "an adapter that records no level must not be failed for it: {report}"
    );
}
