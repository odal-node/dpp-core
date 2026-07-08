//! The evidence dossier wire format — defined once, here, and consumed by
//! both the offline verifier and the engine-side exporter as a dependency.

mod hash;
mod types;

pub use hash::{compute_content_hashes, content_hash};
pub use types::{DossierManifest, DossierV1, SignedLayer};
