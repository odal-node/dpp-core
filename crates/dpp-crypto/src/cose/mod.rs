//! `COSE_Sign1` signing and verification (RFC 9052 §4.2) under EdDSA, ES256, ES384
//! and ES512, the counterpart of [`crate::jws`].
//!
//! The CBOR is read and written by `cbor4ii`, as a codec only. The structure, the
//! `Sig_structure` that is signed, and the header rules are written here, as they
//! are for a JWS: `alg` is bound to the key, `crit` is refused, and `typ` and the
//! content type are returned for the caller to judge. Signing and verification
//! go through the same key types as a JWS, so a key signs under one algorithm in
//! both formats, and ECDSA signatures are made low-`S` and verified either way.
//!
//! The message is read in the tagged form (tag 18) or the untagged one, and written
//! tagged. A detached payload is refused.
//!
//! The CWT Claims header parameter (label 15, RFC 9597) is read from the protected
//! header, and returned as [`CwtClaims`]: the registered claims `iss`, `sub`, `aud`,
//! `exp`, `nbf` and `iat` of RFC 8392. It is refused in the unprotected header, since
//! nothing there is protected. What the claims mean is the caller's: this crate checks
//! their types and returns them, and compares no time and no name.

pub mod algorithm;
mod cbor;
mod cwt;
mod error;
mod header;
mod sign1;

#[cfg(test)]
mod cwt_tests;
#[cfg(test)]
mod fixtures_tests;
#[cfg(test)]
mod header_rule_tests;
#[cfg(test)]
mod message_rule_tests;
#[cfg(test)]
mod proptests;
#[cfg(test)]
mod rfc9052_vector_tests;
#[cfg(test)]
mod round_trip_tests;
#[cfg(test)]
mod suite_tests;
#[cfg(test)]
mod test_support;

pub use algorithm::cose_alg;
pub use cwt::CwtClaims;
pub use error::CoseError;
pub use sign1::{ContentType, VerifiedSign1, sign_sign1, verify_sign1};
