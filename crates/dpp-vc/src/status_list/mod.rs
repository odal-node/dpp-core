//! W3C Bitstring Status List v1.0 — pure decoding + bit indexing.
//!
//! Revocation/suspension of a Verifiable Credential is expressed by a bit in a
//! shared, published "status list" credential. **Fetching** that credential
//! over the network is an infrastructure concern and lives outside core; this
//! module provides only the pure parts so `dpp-crypto` stays infra-free:
//!
//! 1. decode a status list's `encodedList` (multibase base64url of a GZIP-
//!    compressed bitstring) into the raw bitstring, and
//! 2. test the status bit for a holder's index.
//!
//! Source: <https://www.w3.org/TR/vc-bitstring-status-list/>

mod bitstring;
#[cfg(test)]
mod tests;

pub use bitstring::*;
