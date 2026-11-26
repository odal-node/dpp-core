//! The VC-JOSE-COSE corpus an independent JOSE library judges, and what it signs
//! back.
//!
//! The Rust side of `.github/workflows/jose-oracle.yml`. The W3C test suite reads
//! only the `result` an implementation reports about itself, so its issuance rows
//! are self-attested. A JOSE library with no code and no author in common with
//! this crate is the check that is not circular, and for P-521 it is the only one
//! this repository has, since `ring` does not implement that curve.
//!
//! The `cose` modules are the same for the COSE form, and the Rust side of
//! `.github/workflows/cose-oracle.yml`: [`cose::emit`] builds its corpus, [`cose::reverse`]
//! reads what `go-cose` signed, and [`cose::common`] reads a `COSE_Sign1` apart. [`common::keys`] and
//! [`common::high_s`] are shared.
//!
//! - [`emit`] builds the corpus: the suite's issuance inputs, and credentials and
//!   presentations, under EdDSA, ES256, ES384 and ES512, signed with the suite's
//!   own keys. It writes the corpus out when `EMIT_JOSE_CORPUS` is set.
//! - [`reverse`] reads what the library signed, and checks that this crate
//!   verifies it, high-S signatures included.
//! - [`common::keys`] is the suite's four keys, by `kid`.
//! - [`common::token`] reads a compact JWS apart.
//! - [`common::high_s`] is what the order of each curve says about an ECDSA `S`.
//!
//! Without the variables the tests here still run and still check that the corpus
//! is complete and that this crate reads it back, so an ordinary `just check` gets
//! that coverage without producing files.

mod common;
mod cose;
mod emit;
mod reverse;
