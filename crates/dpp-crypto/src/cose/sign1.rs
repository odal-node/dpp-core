//! `COSE_Sign1` (RFC 9052 §4.2): signing, and verification bound to the key.

use cbor4ii::core::utils::SliceReader;

use super::algorithm::cose_alg;
use super::cbor::{self, Out, Reader};
use super::error::CoseError;
use super::header::{self, Alg, Bucket, Media};
use crate::jws::{PublicKey, SignatureLengthError};
use crate::keystore::KeyEntry;

/// The CBOR tag of `COSE_Sign1` (RFC 9052 §4.2).
const SIGN1_TAG: u64 = 18;
/// The first byte of tag 18: major type 6, argument 18 (RFC 8949 §3.4).
const SIGN1_TAG_BYTE: u8 = 0xc0 | 18;
/// The context string of the `Sig_structure` of a `COSE_Sign1` (RFC 9052 §4.4).
const SIGNATURE1_CONTEXT: &str = "Signature1";

/// A content type or `typ` header value (RFC 9052 §3.1; RFC 9596 §2): a media type,
/// or an unsigned integer from the CoAP Content-Formats registry.
///
/// What a value must be is for the caller to judge, as RFC 9596 §2 leaves it to the
/// application.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContentType {
    /// A media type as text, such as `application/vc`.
    Text(String),
    /// An identifier from the CoAP Content-Formats registry.
    CoapFormat(u64),
}

impl ContentType {
    /// The media type, when it is text.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::CoapFormat(_) => None,
        }
    }
}

impl From<Media<'_>> for ContentType {
    fn from(media: Media<'_>) -> Self {
        match media {
            Media::Text(text) => Self::Text(text.to_owned()),
            Media::Unsigned(id) => Self::CoapFormat(id),
        }
    }
}

/// A `COSE_Sign1` message whose signature verified, with the protected header values
/// a caller acts on.
///
/// Every value here comes from the **protected** header, which the signature covers.
/// A `kid` or content type that sits only in the unprotected header is not returned:
/// nothing authenticates it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VerifiedSign1 {
    /// The payload, which the signature covers.
    pub payload: Vec<u8>,
    /// The key identifier (`kid`, label 4), a hint for finding the key: RFC 9052
    /// §3.1 says it "is not a security-critical field".
    pub kid: Option<Vec<u8>>,
    /// The `typ` header parameter (label 16), which the caller compares with the type
    /// it expects.
    pub typ: Option<ContentType>,
    /// The content type of the payload (label 3), which the caller compares too.
    pub content_type: Option<ContentType>,
}

/// Signs `payload` as a tagged `COSE_Sign1` under `key`'s own algorithm (EdDSA,
/// ES256, ES384 or ES512) and returns the encoded message.
///
/// - The protected header holds `alg` (the key's, RFC 9053 §2), the `content_type`
///   (label 3) and `typ` (label 16, RFC 9596) when given, and the `kid` (label 4),
///   in ascending label order. It is a byte string holding a map, and the
///   unprotected header is the empty map (RFC 9052 §3).
/// - The signature is over the `Sig_structure` of RFC 9052 §4.4 with an empty
///   external AAD, encoded as §9 requires.
/// - The signature is the fixed-length `R ‖ S` of RFC 9053 §2.1 for ECDSA, with `S`
///   in the lower half of its range, and RFC 8032's for Ed25519.
/// - The message is written tagged with 18 (RFC 9052 §4.2), the form the W3C
///   VC-JOSE-COSE test suite's inputs have.
///
/// **The `kid` is a parameter, with no default,** as it is for [`crate::jws::sign`]:
/// what it must be depends on how the message is verified.
///
/// # Errors
/// [`CoseError::Encode`], which writing to memory does not raise.
pub fn sign_sign1(
    key: &KeyEntry,
    payload: &[u8],
    kid: &[u8],
    typ: Option<&str>,
    content_type: Option<&str>,
) -> Result<Vec<u8>, CoseError> {
    let protected = encode_protected(key, kid, typ, content_type)?;
    let signature = key.sign(&to_be_signed(&protected, payload)?);

    let mut out = Out::default();
    out.0.push(SIGN1_TAG_BYTE);
    cbor::put_array_header(&mut out, 4)?;
    cbor::put_bytes(&mut out, &protected)?;
    cbor::put_map_header(&mut out, 0)?;
    cbor::put_bytes(&mut out, payload)?;
    cbor::put_bytes(&mut out, &signature)?;
    Ok(out.0)
}

/// The protected header map, encoded: labels ascending, definite lengths, shortest
/// arguments.
fn encode_protected(
    key: &KeyEntry,
    kid: &[u8],
    typ: Option<&str>,
    content_type: Option<&str>,
) -> Result<Vec<u8>, CoseError> {
    let mut out = Out::default();
    cbor::put_map_header(
        &mut out,
        2 + usize::from(typ.is_some()) + usize::from(content_type.is_some()),
    )?;
    cbor::put(&mut out, 1_u8)?;
    cbor::put(&mut out, cose_alg(key.algorithm()))?;
    if let Some(content_type) = content_type {
        cbor::put(&mut out, 3_u8)?;
        cbor::put_text(&mut out, content_type)?;
    }
    cbor::put(&mut out, 4_u8)?;
    cbor::put_bytes(&mut out, kid)?;
    if let Some(typ) = typ {
        cbor::put(&mut out, 16_u8)?;
        cbor::put_text(&mut out, typ)?;
    }
    Ok(out.0)
}

/// The `Sig_structure` of a `COSE_Sign1`, which is what is signed (RFC 9052 §4.4): the
/// array `["Signature1", protected, h'', payload]`, the protected header as the
/// bytes it has in the message, and the external AAD empty.
///
/// It is encoded with definite lengths and the shortest length arguments, as
/// RFC 9052 §9 requires of this structure.
pub(super) fn to_be_signed(protected: &[u8], payload: &[u8]) -> Result<Vec<u8>, CoseError> {
    let mut out = Out::default();
    cbor::put_array_header(&mut out, 4)?;
    cbor::put_text(&mut out, SIGNATURE1_CONTEXT)?;
    cbor::put_bytes(&mut out, protected)?;
    cbor::put_bytes(&mut out, &[])?;
    cbor::put_bytes(&mut out, payload)?;
    Ok(out.0)
}

/// Verifies a `COSE_Sign1` message under `key` and returns its payload and protected
/// header values.
///
/// The message is read in the tagged form (tag 18) or the untagged one, which
/// RFC 9052 §4.2 says depends on context. It is refused unless **all** of these hold:
///
/// - it is one well-formed `COSE_Sign1` array of four items, with nothing after it
///   and no indefinite-length array or map in it;
/// - in each header map, no label appears twice (RFC 9052 §3: "the message MUST be
///   rejected as malformed", compared as values, not as encodings), and every label
///   is an integer or a text string;
/// - no label is in both maps (RFC 9052 §3);
/// - neither header carries `crit`, which this crate does not understand, as the JWS
///   verifier does not (RFC 9052 §3.1);
/// - `typ` is not in the unprotected header (RFC 9596 §2);
/// - the **protected** header carries `alg`, and it is the algorithm of `key` (see
///   [`super::algorithm::cose_alg`]). The key decides the algorithm and the header may
///   only confirm it, so a header cannot move a signature to another algorithm;
/// - the payload is a byte string, not `nil`: a detached payload (RFC 9052 §4.1) is
///   refused, because the credential is the payload;
/// - the signature is a byte string of the length of the key's algorithm, and is its
///   signature over the `Sig_structure` of RFC 9052 §4.4, built from the protected
///   header **as received**. A high-`S` ECDSA signature is accepted, as it is for a
///   JWS: it is lawful ECDSA, so nothing may use signature bytes as an identifier.
///
/// Header parameters this crate does not know are ignored (RFC 9052 §3.1 leaves the
/// application to understand what it needs), and any well-formed CBOR value is
/// stepped over.
///
/// # Errors
/// [`CoseError`], naming the first rule the message breaks.
pub fn verify_sign1(message: &[u8], key: &PublicKey) -> Result<VerifiedSign1, CoseError> {
    let mut reader: Reader<'_> = SliceReader::new(message);

    if cbor::next_is_tag(&mut reader)? && cbor::read_tag(&mut reader)? != SIGN1_TAG {
        return Err(CoseError::WrongTag);
    }
    if cbor::read_array_len(&mut reader, "the COSE_Sign1 array")? != 4 {
        return Err(CoseError::ArrayLength);
    }
    let protected_bytes = cbor::read_bytes(&mut reader, "the protected header")?;
    let unprotected = header::read_bucket(&mut reader)?;
    if cbor::next_is_null(&mut reader)? {
        return Err(CoseError::DetachedPayload);
    }
    let payload = cbor::read_bytes(&mut reader, "the payload")?;
    let signature = cbor::read_bytes(&mut reader, "the signature")?;
    if !cbor::finished(&mut reader) {
        return Err(CoseError::TrailingBytes);
    }

    let protected = read_protected(protected_bytes)?;
    check_headers(&protected, &unprotected, key)?;

    let expected = key.algorithm().signature_len();
    if signature.len() != expected {
        return Err(SignatureLengthError {
            algorithm: key.algorithm(),
            expected,
            actual: signature.len(),
        }
        .into());
    }
    if !key.verify(&to_be_signed(protected_bytes, payload)?, signature) {
        return Err(CoseError::Signature);
    }

    Ok(VerifiedSign1 {
        payload: payload.to_vec(),
        kid: protected.kid.map(<[u8]>::to_vec),
        typ: protected.typ.map(ContentType::from),
        content_type: protected.content_type.map(ContentType::from),
    })
}

/// The protected header map held in the byte string. A zero-length byte string is the
/// empty header (RFC 9052 §3, which also has a recipient accept the empty map).
fn read_protected(bytes: &[u8]) -> Result<Bucket<'_>, CoseError> {
    if bytes.is_empty() {
        return Ok(Bucket::default());
    }
    let mut reader = SliceReader::new(bytes);
    let bucket = header::read_bucket(&mut reader)?;
    if cbor::finished(&mut reader) {
        Ok(bucket)
    } else {
        Err(CoseError::Malformed("bytes after the protected header map"))
    }
}

/// The rules that need both buckets and the key.
fn check_headers(
    protected: &Bucket<'_>,
    unprotected: &Bucket<'_>,
    key: &PublicKey,
) -> Result<(), CoseError> {
    if unprotected.typ.is_some() {
        return Err(CoseError::TypeNotProtected);
    }
    if protected
        .labels
        .intersection(&unprotected.labels)
        .next()
        .is_some()
    {
        return Err(CoseError::LabelInBothBuckets);
    }
    match protected.alg {
        None if unprotected.alg.is_some() => Err(CoseError::AlgorithmNotProtected),
        None => Err(CoseError::MissingAlgorithm),
        Some(Alg::Integer(alg)) if alg == cose_alg(key.algorithm()) => Ok(()),
        Some(_) => Err(CoseError::AlgorithmMismatch),
    }
}
