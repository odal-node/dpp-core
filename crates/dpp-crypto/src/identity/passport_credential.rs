//! Re-export shim: `PassportCredential` and `PassportCredentialSubject` are
//! defined in `dpp-domain` and re-exported here for callers that import from `dpp-crypto`.

pub use dpp_domain::domain::identity::{PassportCredential, PassportCredentialSubject};
