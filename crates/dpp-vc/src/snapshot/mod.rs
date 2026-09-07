//! Continuity snapshots: the time bound on a copy of a public view.
//!
//! A published passport's public view is signed once, at publish, and that
//! proof never expires — which is correct for the passport's *content* and
//! wrong for a *copy* of it. A copy that leaks, is cached, or is mirrored would
//! otherwise keep answering under a signature that stays valid forever.
//!
//! So a snapshot carries a second, outer proof over the whole document
//! (`publicJwsSignature` included) plus an `asOf` and a `validUntil`. The
//! publisher re-signs it on a cadence; withdrawal is simply stopping, and the
//! copy lapses wherever it has got to.
//!
//! **That only works if verification enforces it.** A time bound the verifier
//! ignores is a comment. The check has to travel with verification rather than
//! with the fetch, because the whole point is that a copy nobody is watching
//! expires without anyone's cooperation — and anything a caller has to remember
//! to do is a check that eventually does not happen.

mod bound;
#[cfg(test)]
mod tests;
mod verify;

pub use bound::SnapshotBound;
pub use verify::{CLOCK_SKEW_TOLERANCE, verify_snapshot_bound};
