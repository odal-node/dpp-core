//! No-op ("ghost") port implementations for development and pre-integration use.
//!
//! Each port whose real adapter depends on an external system not yet
//! available at compile time (object storage, the unpublished EU Central
//! Registry API, a QTSP) ships a synthetic implementation here so calling
//! code compiles and runs against a stable contract before the real
//! integration lands. Grouped together because they share one audience —
//! callers wiring a development or standalone deployment — distinct from the
//! port types/trait files, which are addressed to implementers.
//!
//! Private module: each type is re-exported at its own port's module path
//! (`ports::archive::GhostArchive`, `ports::registry_sync::GhostRegistrySync`,
//! `ports::seal::GhostSeal`) and from the crate root, which is the only
//! public way to reach them.
//!
//! **Deviation, accepted:** the pack's `test-doubles` feature (gating these
//! three types behind `#[cfg(feature = "test-doubles")]` so they cannot ship
//! in a production build) was not implemented. These ghosts always compile
//! in; a caller who wires one into a production deployment gets no
//! compile-time signal. The runtime honesty guard (each ghost's `placeholder:
//! true` / `Pending` / synthetic-ID markers) is the sole safeguard. Accepted
//! because a single always-public path per port is simpler to consume and to
//! reason about than a feature-gated one, and the guard is load-bearing
//! either way; revisit only if a ghost is ever caught reaching production
//! silently.

mod archive;
mod registry_sync;
mod seal;
#[cfg(test)]
mod tests;

pub use archive::GhostArchive;
pub use registry_sync::GhostRegistrySync;
pub use seal::GhostSeal;
