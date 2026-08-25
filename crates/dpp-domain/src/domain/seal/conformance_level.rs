//! [`SealConformanceLevel`] — the eIDAS level a seal claims.

use serde::{Deserialize, Serialize};

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
/// rule on [`SealCapabilities::can_produce`](crate::domain::seal::SealCapabilities::can_produce) worth having.
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
    /// Every level this build models. Same reasoning as [`SealFormat::ALL`](crate::domain::seal::SealFormat::ALL).
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
