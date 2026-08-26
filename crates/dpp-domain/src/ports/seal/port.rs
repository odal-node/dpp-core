//! [`SealPort`] — the eIDAS qualified-seal boundary.

use async_trait::async_trait;

use crate::error::DppError;
use crate::seal::{SealCapabilities, SealRequest, SealVerification, SealedEnvelope};

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
    /// If [`SealCapabilities::can_produce`](crate::seal::SealCapabilities::can_produce) is false for `req`, this must return
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
