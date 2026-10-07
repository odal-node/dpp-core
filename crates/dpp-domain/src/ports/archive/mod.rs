//! Port trait for the **archive** of a live passport's historical versions, the
//! functionality EN 18221:2026 clause 4.2 calls archiving.
//!
//! A passport changes. Each time it does, the version it had just before the
//! change is archived, so that the passport as it stood at any earlier moment can
//! be retrieved by those entitled to read it. Archiving starts at the first change
//! and keeps every version from then on, for the passport's lifetime. **Creating a
//! passport archives nothing**: there is no earlier version to keep.
//!
//! # Not the back-up copy
//!
//! [`BackupCopyPort`](crate::ports::backup::BackupCopyPort) holds **one copy of
//! one record**, so that the passport survives its operator (ESPR Art. 10(4)).
//! This port holds **a series of versions of one record**. They are different
//! shapes answering different obligations, and neither implies the other.
//!
//! **The line is drawn by shape, never by actor.** Clause 4.2 expects the archived
//! versions to be held by the back-up provider as well as by the main store, and
//! clause 4.3 has the back-up hold the latest version and the historical ones. So
//! a back-up provider implements this port alongside `BackupCopyPort`, and the
//! main store implements it too. Nothing here exempts a provider from archiving,
//! and nothing on `BackupCopyPort` exempts the main store from keeping a copy.
//!
//! # What the law requires, and what the standard adds
//!
//! ESPR Art. 10(4) asks for a back-up copy, and Arts. 27(1)(c) and 29 make it a
//! copy of the most up-to-date version. The Regulation's own text therefore does
//! not ask the back-up for history. The standard does: its Annex ZA maps
//! Art. 10(4) to clauses 4.3 and 4.5, so a back-up that holds history is what the
//! presumption of conformity under Art. 41(2) costs. Say it that way and no
//! other: it is not a requirement of the Regulation.
//!
//! # The contract
//!
//! - **Archiving starts at the first change.** A caller archives the version a
//!   change replaces, at the moment it replaces it, and archives nothing when it
//!   creates a passport.
//! - **Versions are append-only, and kept for the passport's lifetime.** The port
//!   has no method that changes or removes one, on purpose.
//! - **The archived document is the passport as it stood**, whole and as written.
//!   It is a [`serde_json::Value`] and not a typed `Passport`, because an archive
//!   is evidence: reading a document through a struct drops what the struct does
//!   not know, which changes its bytes, its hash and the signature over it. A
//!   version written under an older shape is read back through the lens machinery
//!   by the caller, never by the port.
//! - **The port returns whole documents and applies no disclosure policy.** The
//!   caller does. Clause 4.2 gives an archived attribute the same access
//!   restriction as the corresponding current one, so the live passport's policy
//!   in force now applies, and serving an archived version without it leaks
//!   exactly what serving the live document without it would.
//! - **Replication.** The main store writes synchronously. A back-up provider may
//!   lag behind it, and clause 4.5 asks that it be kept close. This contract
//!   declares no bound on how far behind is acceptable. What it does give is the
//!   means to measure it: [`ArchiveReceipt::archived_at`] against the version's
//!   `superseded_at`.
//!
//! # The content hash
//!
//! `content_hash` is the lower-case hexadecimal SHA-256 of the RFC 8785 (JCS)
//! canonical form of the document. `BackupReceipt::content_hash` is defined the
//! same way, so a version held here and the back-up copy of that same version
//! carry one hash, and the two can be matched.
//!
//! A registry proof of registration carries a hash of the passport version it
//! covers (CIR (EU) 2026/1778 Art. 9(2)(e)). The Regulation names no algorithm,
//! so which hash a registry uses is not settled by anything held here. That does
//! not make this one wrong: because the port returns whole documents, a caller can
//! derive whichever hash it is asked for from `doc`. The receipt's hash is for
//! integrity, and for matching versions across the two ports.
//!
//! # Not covered
//!
//! - **Transport.** How a provider is reached is the business of standards this
//!   workspace does not hold.
//! - **Access through the back-up once the operator has left the market**
//!   (clause 4.3). That is a separate design question.
//! - **Integrity protection beyond the hash** (EN 18246). The version hash is the
//!   part that can be done now.
//!
//! There is deliberately **no no-op implementation**, unlike the other ports. A
//! ghost that accepted versions and kept none would make a deployment look as if
//! it archived.

mod port;
mod receipt;
#[cfg(any(test, feature = "test-utils"))]
pub mod stub;
#[cfg(test)]
mod tests;
mod version;

pub use port::ArchivedVersionPort;
pub use receipt::ArchiveReceipt;
pub use version::ArchivedVersion;
