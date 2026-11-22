//! Errors of the COSE module.

use crate::jws::SignatureLengthError;

/// Why a `COSE_Sign1` message was refused, or could not be written.
///
/// Every variant names a rule and none carries bytes of the message or of a key, so
/// an error is safe to log. A message that breaks two rules is refused for the
/// first one this crate meets, which is not a promise about which.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CoseError {
    /// The bytes are not the CBOR this structure needs: not well formed, the wrong
    /// type of item in a place, or a map label that is neither an integer nor a text
    /// string (RFC 9052 §1.5). The text says where.
    #[error("not a COSE_Sign1 message: {0}")]
    Malformed(&'static str),
    /// An array, map, byte string or text string that this crate reads in the message
    /// has an indefinite length. RFC 9052 does not forbid reading one; this crate does,
    /// in the structures it reads.
    #[error("an indefinite-length array, map or string in a COSE_Sign1 message")]
    IndefiniteLength,
    /// The message carries a CBOR tag other than 18, the tag of `COSE_Sign1`
    /// (RFC 9052 §4.2).
    #[error("a CBOR tag other than 18 before the COSE_Sign1 array")]
    WrongTag,
    /// The array does not have the four items of `COSE_Sign1` (RFC 9052 §4.2).
    #[error("a COSE_Sign1 array must have four items")]
    ArrayLength,
    /// Bytes follow the one CBOR item that is the message.
    #[error("bytes after the COSE_Sign1 message")]
    TrailingBytes,
    /// A label appears twice in one header map. RFC 9052 §3 says the message "MUST
    /// be rejected as malformed".
    #[error("a header label appears twice in one map")]
    DuplicateLabel,
    /// A label is in both the protected and the unprotected header. RFC 9052 §3
    /// says applications SHOULD check for this; this crate does.
    #[error("a header label is in both the protected and the unprotected header")]
    LabelInBothBuckets,
    /// A header carries `crit` (label 2). This crate understands no critical
    /// header parameter, so none is accepted, as the JWS verifier refuses `crit`.
    #[error("a header names critical parameters, none of which this crate understands")]
    Critical,
    /// The `typ` header parameter (label 16) is in the unprotected header, where
    /// RFC 9596 §2 says it MUST NOT be.
    #[error("the typ header parameter is in the unprotected header")]
    TypeNotProtected,
    /// The CWT Claims header parameter (label 15) is in the unprotected header. RFC 9597 §1
    /// says recipients "MUST NOT use the information in the CWT Claims header parameter
    /// beyond the integrity protection or authentication afforded to the CWT Claims
    /// header", and nothing protects the unprotected header, so no claim there is read.
    #[error("the CWT Claims header parameter is in the unprotected header")]
    CwtClaimsNotProtected,
    /// The protected header carries no `alg`, and neither does the unprotected one.
    #[error("the protected header carries no alg")]
    MissingAlgorithm,
    /// `alg` is only in the unprotected header, where nothing authenticates it.
    #[error("alg is in the unprotected header only")]
    AlgorithmNotProtected,
    /// The protected `alg` is not the algorithm of the key the message is checked
    /// against. The key decides the algorithm and the header may only confirm it.
    #[error("the protected alg is not the algorithm of the key")]
    AlgorithmMismatch,
    /// The payload is `nil`, so the message is detached (RFC 9052 §4.1). The
    /// credential is the payload, so there is nothing to return.
    #[error("the payload is detached")]
    DetachedPayload,
    /// The signature is not the length its algorithm uses.
    #[error(transparent)]
    SignatureLength(#[from] SignatureLengthError),
    /// The signature is not the key's signature over the `Sig_structure`.
    #[error("the signature does not verify")]
    Signature,
    /// Writing the message failed. The writer is memory, so this is not reached,
    /// and it is kept so that no path panics.
    #[error("the COSE_Sign1 message could not be written")]
    Encode,
    /// The signing key is revoked, and a revoked key signs nothing.
    #[error(transparent)]
    KeyRevoked(#[from] crate::keystore::KeyRevoked),
}
