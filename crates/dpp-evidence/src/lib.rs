//! Evidence dossier format and offline verification for EU DPP — the proof
//! surface of the Odal Node standard.
//!
//! A dossier ([`DossierV1`]) is a self-contained, signed export of a
//! passport's full proof chain — JWS signatures, hash-chained audit trail,
//! transfer-chain signatures — verifiable by anyone with zero trust in the
//! issuing node. This crate defines the wire format once and provides the
//! verification engine both the engine-side exporter (`dpp-vault`) and the
//! `odal verify` CLI command (`dpp-engine`) depend on.
//!
//! Deliberately free of any BSL-licensed or wasm-unsafe dependency — see
//! [`jws`]'s module doc for what is vendored from `dpp-crypto` (and why) and
//! [`audit`]'s module doc for what was promoted from `dpp-engine`'s
//! `dpp-types` crate.
//!
//! See `spec/dossier-v1.md` for the full format specification.

pub mod audit;
pub mod dossier;
pub mod jws;
pub mod verify;

pub use audit::AuditEntry;
pub use dossier::{DossierManifest, DossierV1, SignedLayer, compute_content_hashes, content_hash};
pub use verify::{
    CheckResult, CheckStatus, DossierParseError, VerificationReport, VerifyMode, did_web_url,
    verify_dossier, verify_dossier_json,
};
