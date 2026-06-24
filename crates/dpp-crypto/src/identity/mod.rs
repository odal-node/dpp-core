//! DID / identity orchestration — `did:web` document builder and `IdentityPort` impl.

pub mod did_builder;
pub mod local_service;
pub(crate) mod passport_credential;
#[cfg(test)]
mod tests;

pub use did_builder::build_did_document;
pub use local_service::LocalIdentityService;
pub use passport_credential::{PassportCredential, PassportCredentialSubject};
