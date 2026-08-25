//! Port trait for ESPR-mandated third-party DPP archival.
//!
//! EU ESPR requires that DPP data remains accessible for the period defined
//! in the applicable delegated act, even in cases of insolvency or market
//! withdrawal by the economic operator. A copy of the DPP must be hosted by
//! an independent third-party digital service provider.
//!
//! The obligation is **Art. 10(4)**: the economic operator "shall make available
//! a back-up copy of the digital product passport through a digital product
//! passport service provider", which **Art. 2(32)** defines as "an independent
//! third-party authorised by the economic operator". The period is **Annex
//! III(i)** — "at least the expected lifetime of a specific product" — delegated
//! per product group. **Annex III(l)** makes the provider's reference a passport
//! data element.
//!
//! Two consequences worth stating, because both have been got wrong before.
//! *Independent third party* means an operator's own storage does not discharge
//! this, however durable. And the article is **not Art. 13**, which establishes
//! the registry and is a different duty entirely.
//!
//! This port defines the contract that platform adapters implement to
//! replicate published passport data to an independent archive.

mod port;
mod receipt;
mod status;
#[cfg(any(test, feature = "test-utils"))]
pub mod stub;
#[cfg(test)]
mod tests;
mod verification;

pub use port::ArchivePort;
pub use receipt::ArchiveReceipt;
pub use status::ArchiveStatus;
pub use verification::ArchiveVerification;

pub(crate) use receipt::retention_deadline;

/// No-op archive for development and standalone vault deployments.
///
/// All operations succeed without performing any I/O. Returns synthetic
/// receipts with `archive_id = "ghost-{uuid}"`. Use in tests and in the
/// standalone `dpp-vault` binary where object storage is not configured.
pub use crate::ports::ghosts::GhostArchive;
