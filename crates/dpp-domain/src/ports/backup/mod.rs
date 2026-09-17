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
//! still live* — a **history**, many versions of one record. This port is a
//! **copy**, one version of one record, held by somebody else so that the
//! passport survives its operator. Different shapes, answering different
//! obligations: clause 4.2, and ESPR Art. 10(4).
//!
//! **The line is drawn by shape, not by actor, and getting that backwards is
//! the easy mistake.** Clause 4.2 expects a passport's archived versions to be
//! held by the back-up provider as well as by the main one, so "the provider
//! owes no history" is *false* — a provider may well owe one. What is true is
//! narrower and is about this trait: nothing on it takes or returns a series,
//! so a history is not expressible here whoever owes it. While the two shared
//! a word, a reader could satisfy themselves that wiring this port had ticked
//! clause 4.2. Wiring it cannot, and that is a statement about this interface
//! rather than about anybody's duties.
//!
//! The Regulation supplies the name. ESPR Art. 10(4) says *back-up copy*, so that is
//! what this is called, and "archiving" is left to mean only the thing the
//! standard means by it. See [`PassportStatus`](crate::status::PassportStatus)
//! for the third use the word had — a lifecycle status, now `Retired`.
//!
//! The obligation is **ESPR Art. 10(4)**: the economic operator "shall make available
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
