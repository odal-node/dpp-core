//! Port trait for eIDAS qualified electronic sealing.
//!
//! ESPR Article 13 establishes the EU Central Registry, which became
//! operational on **20 July 2026**; its operating rules are Commission
//! Implementing Regulation (EU) 2026/1778.
//!
//! Verified against the OJ text of IR 2026/1778: for a legal person, verified
//! status rests on a **qualified electronic seal** supported by a qualified
//! certificate issued by a QTSP, or on a qualified electronic attestation of
//! attributes (**Art. 4(2)** for economic operators, **Art. 5(2)** for value
//! chain actors). Establishment in the Union is not a precondition — both
//! articles provide expressly for parties not required to be so established.
//!
//! ## What "qualified" actually requires
//!
//! Verified against the OJ text of Regulation (EU) No 910/2014. A **qualified
//! electronic seal** is a three-part conjunction (**Art. 3(27)**):
//!
//! 1. an *advanced* electronic seal (**Art. 36**), **and**
//! 2. created by a *qualified electronic seal creation device* (**Art. 3(32)**),
//!    **and**
//! 3. based on a *qualified certificate* issued by a QTSP (**Art. 3(30)**,
//!    **Annex III**).
//!
//! An adapter that produces an advanced seal over a qualified certificate does
//! **not** satisfy this: the creation-device leg is separate and independently
//! required. **Annex III(j)** makes it machine-detectable — the certificate must
//! indicate, in a form suitable for automated processing, that the creation data
//! resides in such a device. The payoff is **Art. 35(2)**: a qualified seal
//! carries a presumption of integrity of the data and of correctness of its
//! origin.
//!
//! Two operating models exist:
//! - **Provider seal (delegated):** the platform holds its own qualified seal;
//!   operators register via delegated access without their own eIDAS credentials.
//! - **Operator seal:** the operator obtains and manages their own qualified seal.
//!
//! The real adapter calls a QTSP over the CSC API (Cloud Signature Consortium)
//! and lives in `dpp-engine`. Until a QTSP integration is configured,
//! `GhostSeal` returns clearly-synthetic envelopes so registration code can be
//! written and tested against this contract today.
//!
//! The value objects this trait produces — [`SealedEnvelope`] and friends — are
//! domain values, not ports, and live in [`crate::domain::seal`]. They are
//! re-exported here so existing paths keep resolving.

use async_trait::async_trait;

use crate::domain::error::DppError;

pub use crate::domain::seal::{
    SealCapabilities, SealChecks, SealCredentialRef, SealFormat, SealIndication, SealMode,
    SealRequest, SealVerification, SealedEnvelope,
};

// ─── Port Trait ──────────────────────────────────────────────────────────────

/// Port trait for applying and verifying eIDAS qualified electronic seals.
///
/// Implementations live in `dpp-engine` and call a QTSP over the CSC API.
/// Until a QTSP is configured, wire `GhostSeal` so registration code compiles
/// and runs against a stable contract.
#[async_trait]
pub trait SealPort: Send + Sync {
    /// Apply a qualified seal to the given payload hash.
    ///
    /// # An implementation must refuse what it cannot produce
    ///
    /// If [`SealCapabilities::can_produce`] is false for `req`, this must return
    /// an error rather than a seal. Advertising one thing and delivering another
    /// is not a convenience — it silently substitutes a different attestation
    /// than the caller asked for, and both axes carry meaning that cannot be
    /// swapped: the format decides what a verifier must run, and the mode decides
    /// *whose* attestation the seal is.
    ///
    /// Sealing is normally irreversible in practice — the seal is bought, and
    /// the document it covers is retention-locked — so a substitution discovered
    /// later cannot be undone by re-sealing. Refusing costs a failed request;
    /// accepting costs an attestation that says something nobody chose.
    async fn seal(&self, req: SealRequest) -> Result<SealedEnvelope, DppError>;

    /// Verify a previously produced seal envelope.
    async fn verify(&self, env: &SealedEnvelope) -> Result<SealVerification, DppError>;

    /// Report which formats and modes this adapter supports.
    fn capabilities(&self) -> SealCapabilities;
}

// ─── Ghost implementation (development / pre-QTSP) ───────────────────────────

/// No-op implementation for use before a QTSP integration is configured.
///
/// Returns synthetic envelopes marked `placeholder: true`. All operations
/// succeed but perform no network I/O and carry no legal validity.
pub use crate::ports::ghosts::GhostSeal;
