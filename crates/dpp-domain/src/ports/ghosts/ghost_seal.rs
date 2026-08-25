//! [`GhostSeal`] — a no-op seal port that produces an unsigned envelope.

use async_trait::async_trait;
use chrono::Utc;

use crate::error::dpp::DppError;

use crate::ports::seal::{
    SealCapabilities, SealChecks, SealConformanceLevel, SealEnvelope, SealFormat, SealIndication,
    SealMode, SealPort, SealRequest, SealVerification, SealedEnvelope,
};

/// No-op implementation for use before a QTSP integration is configured.
///
/// Returns synthetic envelopes marked `placeholder: true`. All operations
/// succeed but perform no network I/O and carry no legal validity.
pub struct GhostSeal;

#[async_trait]
impl SealPort for GhostSeal {
    async fn seal(&self, req: SealRequest) -> Result<SealedEnvelope, DppError> {
        // The ghost is held to the same obligation as a real adapter. It used to
        // echo whichever format it was handed while advertising one — so asking
        // it for CAdES produced a "CAdES" envelope from an adapter claiming to
        // support only JAdES. Harmless in itself, since nothing here is a real
        // seal, but it made the ghost useless for catching that mistake in a
        // consumer, which is most of what a ghost is for.
        if !self.capabilities().can_produce(&req) {
            return Err(DppError::Validation(
                crate::error::field::ValidationErrors::message(format!(
                    "GhostSeal does not produce {:?}/{:?}",
                    req.sig_format, req.mode
                )),
            ));
        }
        Ok(SealedEnvelope {
            format: req.sig_format,
            seal_value: format!(
                "GHOST-SEAL-{}",
                &req.payload_hash[..8.min(req.payload_hash.len())]
            ),
            signing_cert_ref: None,
            sealed_at: Utc::now(),
            placeholder: true,
        })
    }

    async fn verify(&self, env: &SealedEnvelope) -> Result<SealVerification, DppError> {
        Ok(SealVerification {
            // Not `TotalFailed`: nothing about this envelope was checked, so
            // calling it invalid would be a verdict the ghost did not reach.
            // A placeholder is precisely the indeterminate case — there is
            // nothing here to validate, and saying so is the honest answer.
            indication: SealIndication::Indeterminate(
                "placeholder seal: no validation was performed and none is possible".to_owned(),
            ),
            checks: SealChecks::None,
            placeholder: env.placeholder,
        })
    }

    fn capabilities(&self) -> SealCapabilities {
        SealCapabilities {
            // Every format, because a placeholder genuinely can fabricate any of
            // them — this is what the ghost does, stated accurately, rather than
            // an arbitrary subset it then failed to honour.
            //
            // Enumerated rather than "all": `SealFormat` is `#[non_exhaustive]`,
            // so a format added later is deliberately *not* covered here. A new
            // envelope format should have to be admitted on purpose, including
            // for the ghost.
            supported_formats: vec![
                SealFormat::Jades,
                SealFormat::Pades,
                SealFormat::Cades,
                SealFormat::Xades,
            ],
            supported_modes: vec![SealMode::ProviderSeal, SealMode::OperatorSeal],
            // A placeholder can fabricate any baseline level as readily as any
            // other — none of it is real. Advertising all four keeps the ghost
            // useful for exercising a consumer that asks for long-term validity,
            // which is the case a real provider is most likely to refuse.
            supported_levels: vec![
                SealConformanceLevel::BaselineB,
                SealConformanceLevel::BaselineT,
                SealConformanceLevel::BaselineLt,
                SealConformanceLevel::BaselineLta,
            ],
            // Every packaging, for the same reason as the formats — and because
            // advertising all four formats while omitting any one format's
            // packagings would leave that format unrequestable, which is a
            // stranger thing for a ghost to claim than fabricating them all.
            // Enumerated rather than `SealEnvelope::ALL` on the same principle.
            supported_envelopes: vec![
                SealEnvelope::Detached,
                SealEnvelope::Enveloping,
                SealEnvelope::Attached,
                SealEnvelope::Parallel,
                SealEnvelope::Enveloped,
                SealEnvelope::Certification,
                SealEnvelope::Revision,
            ],
        }
    }
}
