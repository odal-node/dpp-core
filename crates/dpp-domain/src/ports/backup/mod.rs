//! Port trait for the ESPR-mandated third-party **back-up copy**.
//!
//! EU ESPR requires that DPP data remains accessible for the period defined
//! in the applicable delegated act, even in cases of insolvency or market
//! withdrawal by the economic operator. A copy of the DPP must be hosted by
//! an independent third-party digital service provider.
//!
//! # Why this is not called an archive
//!
//! It was, and the word was wanted elsewhere. **EN 18221:2026 clause 4.2**
//! archiving is the retention of *historical versions of a passport that is
//! still live* — a history of one record. This is a *copy* of one record, held
//! by somebody else so that it survives the operator. The two answer different
//! obligations and fail in different ways, and while they shared a word, a
//! reader could satisfy themselves that this port discharged clause 4.2. It
//! does not, and cannot: it replicates the current record, so it has no past
//! versions to serve.
//!
//! The Regulation supplies the name. Art. 10(4) says *back-up copy*, so that is
//! what this is called, and "archiving" is left to mean only the thing the
//! standard means by it. See [`PassportStatus`](crate::status::PassportStatus)
//! for the third use the word had — a lifecycle status, now `Retired`.
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
//! replicate published passport data to an independent provider.

mod port;
mod receipt;
mod status;
#[cfg(any(test, feature = "test-utils"))]
pub mod stub;
#[cfg(test)]
mod tests;
mod verification;

pub use port::BackupCopyPort;
pub use receipt::BackupReceipt;
pub use status::BackupStatus;
pub use verification::BackupVerification;

pub(crate) use receipt::retention_deadline;

/// No-op back-up copy for development and standalone deployments.
///
/// All operations succeed without performing any I/O. Returns synthetic
/// receipts with `backup_id = "GHOST-{uuid}"`. Use in tests and wherever no
/// provider is configured.
pub use crate::ports::ghosts::GhostBackup;
