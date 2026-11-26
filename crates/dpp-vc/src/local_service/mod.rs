//! In-process `IdentityPort` implementation backed by a local `KeyStore`.

mod local_identity_service;
#[cfg(test)]
mod tests;

pub use local_identity_service::*;
