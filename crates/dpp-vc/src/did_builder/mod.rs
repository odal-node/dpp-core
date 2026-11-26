//! `did:web` DID document builder.
//!
//! Constructs a W3C DID document from an issuer's `KeyStore`: primary key first,
//! hygiene-archived keys as secondary verification methods, revoked keys excluded
//! so their signatures stop verifying.
//!
//! # A key's identifier is its thumbprint
//!
//! A verification method is identified by `did:web:<host>#<thumbprint URI>`, the
//! thumbprint being the RFC 7638 one of the key's JWK in the RFC 9278 URI form.
//! The identifier is a function of the key and nothing else, so it is the same
//! before and after a rotation and for every reader. A position in a list would
//! not be: the identifiers of archived keys used to be renumbered whenever a
//! revoked one dropped out, so a token that named `#key-1` named a different key
//! after the next rotation. The `kid` in a signed token's protected header is
//! fixed when it is issued, so the identifier it names has to be fixed too.
//!
//! The verification methods are of type `JsonWebKey`, defined by W3C Controlled
//! Identifiers v1.0 together with `Multikey`. DID 1.1 defines its key properties
//! by reference to that specification.

mod document;

pub use document::*;
