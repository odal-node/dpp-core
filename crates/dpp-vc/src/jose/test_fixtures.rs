//! The test suite's input files and the documents and tokens this module's own tests
//! build, shared by its test files.
//!
//! The files are the inputs of the W3C VC-JOSE-COSE test suite, carried verbatim:
//! see `fixtures_tests` for where they come from and the pin on each. Each is
//! embedded here once, so no test reads from the file system. The rest builds a
//! credential and a presentation under a key the test generated, and forges tokens
//! with a genuine signature over a header or payload that `sign_credential` would
//! never write, so that a refusal can only be the check it names.

use base64::Engine as _;
use serde_json::{Value, json};

use dpp_crypto::jws::{KeyAlgorithm, PublicKey};
use dpp_crypto::keystore::KeyEntry;

use crate::test_support::temp_store_with;

/// Every vendored file, by its name in the suite.
pub(super) const FILES: [(&str, &str); 19] = [
    (
        "credential-minimal.json",
        include_str!("fixtures/w3c-vc-jose-cose/credential-minimal.json"),
    ),
    (
        "credential-full.json",
        include_str!("fixtures/w3c-vc-jose-cose/credential-full.json"),
    ),
    (
        "presentation-single.json",
        include_str!("fixtures/w3c-vc-jose-cose/presentation-single.json"),
    ),
    (
        "presentation-multiple.json",
        include_str!("fixtures/w3c-vc-jose-cose/presentation-multiple.json"),
    ),
    (
        "credential-unknown-extensions.json",
        include_str!("fixtures/w3c-vc-jose-cose/credential-unknown-extensions.json"),
    ),
    (
        "credential-jose-minimal.txt",
        include_str!("fixtures/w3c-vc-jose-cose/credential-jose-minimal.txt"),
    ),
    (
        "presentation-jose-multiple.txt",
        include_str!("fixtures/w3c-vc-jose-cose/presentation-jose-multiple.txt"),
    ),
    (
        "credential-issuer-match-signed.txt",
        include_str!("fixtures/w3c-vc-jose-cose/credential-issuer-match-signed.txt"),
    ),
    (
        "credential-jose-unknown-extensions.txt",
        include_str!("fixtures/w3c-vc-jose-cose/credential-jose-unknown-extensions.txt"),
    ),
    (
        "credential-jose-bad-signature.txt",
        include_str!("fixtures/w3c-vc-jose-cose/credential-jose-bad-signature.txt"),
    ),
    (
        "credential-jose-bad-media-type.txt",
        include_str!("fixtures/w3c-vc-jose-cose/credential-jose-bad-media-type.txt"),
    ),
    (
        "presentation-jose-bad-media-type.txt",
        include_str!("fixtures/w3c-vc-jose-cose/presentation-jose-bad-media-type.txt"),
    ),
    (
        "credential-jose-vc-vp-claims.txt",
        include_str!("fixtures/w3c-vc-jose-cose/credential-jose-vc-vp-claims.txt"),
    ),
    (
        "presentation-jose-bad-credential.txt",
        include_str!("fixtures/w3c-vc-jose-cose/presentation-jose-bad-credential.txt"),
    ),
    (
        "vm-ed25519.json",
        include_str!("fixtures/w3c-vc-jose-cose/vm-ed25519.json"),
    ),
    (
        "vm-p256.json",
        include_str!("fixtures/w3c-vc-jose-cose/vm-p256.json"),
    ),
    (
        "vm-p384.json",
        include_str!("fixtures/w3c-vc-jose-cose/vm-p384.json"),
    ),
    (
        "vm-p521.json",
        include_str!("fixtures/w3c-vc-jose-cose/vm-p521.json"),
    ),
    (
        "LICENSE.md",
        include_str!("fixtures/w3c-vc-jose-cose/LICENSE.md"),
    ),
];

/// The text of the vendored file `name`.
pub(super) fn file(name: &str) -> &'static str {
    FILES
        .iter()
        .find(|(file, _)| *file == name)
        .unwrap_or_else(|| panic!("{name} is not a vendored file"))
        .1
}

/// A vendored JSON file, parsed.
pub(super) fn json(name: &str) -> Value {
    serde_json::from_str(file(name)).unwrap_or_else(|e| panic!("{name} is not JSON: {e}"))
}

/// A vendored token. A file may end in a newline, which is no part of the token.
pub(super) fn token(name: &str) -> String {
    file(name).trim_end().to_owned()
}

/// The key file of `algorithm`: `p256`, `p384`, `p521` or `ed25519`.
pub(super) fn key_file(algorithm: &str) -> Value {
    json(&format!("vm-{algorithm}.json"))
}

/// The public key of the key file of `algorithm`.
pub(super) fn public_key(algorithm: &str) -> PublicKey {
    PublicKey::from_jwk(&key_file(algorithm)["publicKeyJwk"]).expect("a public key")
}

/// The signing key of the key file of `algorithm`.
pub(super) fn signing_key(algorithm: &str) -> KeyEntry {
    KeyEntry::from_private_jwk(&key_file(algorithm)["secretKeyJwk"]).expect("a private key")
}

/// The `id` of the key file of `algorithm`: the absolute URL of its verification
/// method, which is the `kid` the suite's issuer would sign under.
pub(super) fn key_id(algorithm: &str) -> String {
    key_file(algorithm)["id"]
        .as_str()
        .expect("an id")
        .to_owned()
}

/// The `kid` the tests sign under: an absolute URL.
pub(super) const KID: &str = "https://issuer.example/keys#1";

/// A key this test generated, of `algorithm`.
pub(super) fn generated(algorithm: KeyAlgorithm) -> KeyEntry {
    temp_store_with("jose", "k", algorithm)
        .load_key("k")
        .expect("the key was just generated")
}

/// A credential that uses only base-vocabulary terms.
pub(super) fn credential() -> Value {
    json!({
        "@context": ["https://www.w3.org/ns/credentials/v2"],
        "id": "urn:uuid:7b0a4c1e-3f5d-4c8e-9a41-2d6f8e0b1c33",
        "type": ["VerifiableCredential"],
        "issuer": "https://issuer.example",
        "validFrom": "2025-01-01T00:00:00Z",
        "credentialSubject": {"id": "did:example:subject"},
    })
}

/// The protected header `sign_credential` writes for `key`.
pub(super) fn credential_header(key: &KeyEntry) -> Value {
    json!({"alg": key.algorithm().jose_alg(), "kid": KID, "typ": "vc+jwt", "cty": "vc"})
}

/// The protected header `sign_presentation` writes for `key`.
pub(super) fn presentation_header(key: &KeyEntry) -> Value {
    json!({"alg": key.algorithm().jose_alg(), "kid": KID, "typ": "vp+jwt", "cty": "vp"})
}

/// A compact JWS with a genuine signature by `key` over `header` and `payload`.
pub(super) fn forge(key: &KeyEntry, header: &Value, payload: &Value) -> String {
    let payload = serde_json::to_vec(payload).expect("serialises");
    forge_raw(
        key,
        header,
        &base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload),
    )
}

/// [`forge`] with the payload segment given as text, for a payload that is not a
/// JSON object.
pub(super) fn forge_raw(key: &KeyEntry, header: &Value, payload_segment: &str) -> String {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header = b64.encode(serde_json::to_vec(header).expect("serialises"));
    let input = format!("{header}.{payload_segment}");
    format!("{input}.{}", b64.encode(key.sign(input.as_bytes())))
}

/// An Enveloped Verifiable Credential holding `token` as a `vc+jwt`.
pub(super) fn envelope(token: &str) -> Value {
    json!({
        "@context": "https://www.w3.org/ns/credentials/v2",
        "type": "EnvelopedVerifiableCredential",
        "id": format!("data:application/vc+jwt,{token}"),
    })
}

/// An Enveloped Verifiable Credential of another media type, with data this crate
/// never reads.
pub(super) fn foreign_envelope(media_type: &str) -> Value {
    json!({
        "@context": "https://www.w3.org/ns/credentials/v2",
        "type": "EnvelopedVerifiableCredential",
        "id": format!("data:{media_type},opaque"),
    })
}

/// A presentation of `envelopes`, using only base-vocabulary terms.
pub(super) fn presentation(envelopes: Vec<Value>) -> Value {
    json!({
        "@context": ["https://www.w3.org/ns/credentials/v2"],
        "id": "urn:uuid:0d9c6b0e-5a3c-4f43-8f0e-6f2a8d1c9e55",
        "type": ["VerifiablePresentation"],
        "holder": "https://holder.example",
        "verifiableCredential": envelopes,
    })
}

/// A forged credential token whose protected header `change` has altered.
pub(super) fn with_header(key: &KeyEntry, change: impl FnOnce(&mut Value)) -> String {
    let mut header = credential_header(key);
    change(&mut header);
    forge(key, &header, &credential())
}

/// A forged credential token whose payload `change` has altered.
pub(super) fn with_credential(key: &KeyEntry, change: impl FnOnce(&mut Value)) -> String {
    let mut document = credential();
    change(&mut document);
    forge(key, &credential_header(key), &document)
}
