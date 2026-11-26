//! Helpers the JAdES tests share.

use base64::Engine;
use serde_json::Value;

use super::super::*;

pub(super) const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// A stand-in for a DER certificate. The bytes are never parsed by this module
/// — `x5t#S256` digests them and `x5c` carries them — so a fixed blob is enough
/// to exercise the format without pulling in a certificate generator.
pub(super) const FAKE_DER: &[u8] = b"\x30\x82\x01\x0a-not-a-real-certificate-";

pub(super) fn header() -> JadesHeader {
    JadesHeader {
        alg: "EdDSA".into(),
        iat: 1_770_000_000,
        certificate: CertificateRef::thumbprint_of_der(FAKE_DER),
        content_type: None,
    }
}

pub(super) fn decode_header(compact: &str) -> Value {
    let seg = compact.split('.').next().expect("has a header segment");
    serde_json::from_slice(&B64.decode(seg).expect("header is base64url")).expect("header is JSON")
}
