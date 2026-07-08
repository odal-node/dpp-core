//! The dossier verification engine, its report types, and the two support
//! checks that need more than a single-crate view: whole transfer-chain
//! signature verification, and `did:web` DID-to-URL resolution.

mod did_web;
mod engine;
mod report;
mod transfer_chain;

pub use did_web::did_web_url;
pub use engine::{DossierParseError, verify_dossier, verify_dossier_json};
pub use report::{CheckResult, CheckStatus, VerificationReport, VerifyMode};
pub use transfer_chain::{TransferChainBreak, TransferSignatureIssue, verify_transfer_chain};
