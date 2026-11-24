//! Shared `#[cfg(test)]`-only fixtures for this crate's test suites.
//!
//! The VC-JOSE-COSE fixtures are the input files of the W3C VC-JOSE-COSE test suite,
//! carried verbatim: see `jose::fixtures_tests` for where they come from and the pin on
//! each. Each is embedded here once, so no test reads from the file system. The rest
//! builds a credential and a presentation under a key the test generated, and forges
//! tokens with a genuine signature over a header or payload that `sign_credential`
//! would never write, so that a refusal can only be the check it names.

use base64::Engine as _;
use serde_json::{Value, json};

use dpp_crypto::jws::{KeyAlgorithm, PublicKey};
use dpp_crypto::keystore::{KeyBackend, KeyEntry, KeyStore};

use crate::jose::{KeyBindingRequirement, VerifyOptions};

/// Every algorithm a key can be generated under, in the order a test loops over.
pub(crate) const EVERY_ALGORITHM: [KeyAlgorithm; 4] = [
    KeyAlgorithm::Ed25519,
    KeyAlgorithm::EcdsaP256,
    KeyAlgorithm::EcdsaP384,
    KeyAlgorithm::EcdsaP521,
];

/// A fresh [`KeyStore`] at a unique temp path, with one key already generated.
///
/// Duplicated from `dpp-crypto`'s own test support rather than shared: a
/// `#[cfg(test)]` module is private to its crate, and exposing it across a
/// crate boundary would mean shipping test scaffolding in the published API.
pub(crate) fn temp_store(label: &str, key_id: &str) -> KeyStore {
    temp_store_with(label, key_id, KeyAlgorithm::Ed25519)
}

/// [`temp_store`], with a key of `algorithm` in place of an Ed25519 one.
pub(crate) fn temp_store_with(label: &str, key_id: &str, algorithm: KeyAlgorithm) -> KeyStore {
    let path = std::env::temp_dir().join(format!("test-{label}-{}.json", uuid::Uuid::now_v7()));
    let store = KeyStore::open(path, "test-pass").expect("open store");
    store
        .generate_key_with(key_id, algorithm)
        .expect("generate key");
    store
}

// ── Signing while the key rotates ─────────────────────────────────────────────

/// How many rotations [`sign_while_rotating`] waits for while it signs.
const ROTATIONS_WHILE_SIGNING: usize = 20;

/// How many threads [`sign_while_rotating`] signs on at once.
const SIGNERS: usize = 4;

/// Sign with `sign` on [`SIGNERS`] threads while another thread rotates `key_id` in
/// `store`, and return every token once each thread has seen [`ROTATIONS_WHILE_SIGNING`]
/// rotations complete after its own first token.
///
/// For the signers that take the `kid` from the key: each must take it and the
/// signature from one load of the key, or a rotation between the two reads gives a
/// token the replaced key's `kid` and the new key's signature. A rotation holds the
/// store's write lock while it saves the file, so one thread signing alone is nearly
/// always past both reads when the next rotation takes it; with several signing at
/// once, some are between the two. Each rotation is a [`KeyStore::rotate_key`],
/// which archives the replaced key, so every key that signed a token is still
/// published when the rotations stop.
///
/// Signing starts once the first rotation has completed. Each thread counts from its own
/// first token, so a thread that starts late still signs on both sides of a rotation,
/// and so tokens under more than one key are not left to the scheduler. The test fails
/// if a rotation fails or a signer panics.
pub(crate) fn sign_while_rotating<T: Send>(
    store: &dyn KeyBackend,
    key_id: &str,
    sign: impl Fn() -> T + Sync,
) -> Vec<T> {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    /// Raises the flag when dropped, so the signers stop however the rotating thread
    /// ends, a failed rotation included, and its panic then reaches the test.
    struct StopOnDrop<'a>(&'a AtomicBool);
    impl Drop for StopOnDrop<'_> {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    let stop = AtomicBool::new(false);
    let rotations = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let (stop, rotations, sign) = (&stop, &rotations, &sign);
        scope.spawn(move || {
            let _stop_signers = StopOnDrop(stop);
            while !stop.load(Ordering::SeqCst) {
                store.rotate_key(key_id).expect("rotate the key");
                rotations.fetch_add(1, Ordering::SeqCst);
            }
        });
        while rotations.load(Ordering::SeqCst) == 0 && !stop.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }
        let signers: Vec<_> = (0..SIGNERS)
            .map(|_| {
                scope.spawn(move || {
                    // Counted from this thread's own first token, so however late the
                    // thread starts, its tokens span the rotations that follow it.
                    let mut signed = vec![sign()];
                    let first = rotations.load(Ordering::SeqCst);
                    while !stop.load(Ordering::SeqCst)
                        && rotations.load(Ordering::SeqCst) - first < ROTATIONS_WHILE_SIGNING
                    {
                        signed.push(sign());
                    }
                    signed
                })
            })
            .collect();
        // Every signer is joined before the rotations stop, and they stop before a
        // signer's panic is passed on, so the scope never waits on a rotating thread.
        let signed: Vec<_> = signers.into_iter().map(|s| s.join()).collect();
        stop.store(true, Ordering::SeqCst);
        signed
            .into_iter()
            .flat_map(|s| s.expect("a signer panicked"))
            .collect()
    })
}

// ── VC-JOSE-COSE verification choices ─────────────────────────────────────────

/// Key Binding is not required, which is the choice most tests make: the one under test is
/// another.
pub(crate) const NOT_REQUIRED: KeyBindingRequirement = KeyBindingRequirement::NotRequired;

/// The options of a test that gives a `now` or none, and no audience.
pub(crate) fn at(now: Option<i64>) -> VerifyOptions<'static> {
    now.map_or_else(VerifyOptions::new, |now| VerifyOptions::new().at(now))
}

// ── VC-JOSE-COSE suite inputs and fixtures ────────────────────────────────────

/// Every vendored file, by its name in the suite.
pub(crate) const FILES: [(&str, &str); 37] = [
    (
        "credential-minimal.json",
        include_str!("jose/fixtures/w3c-vc-jose-cose/credential-minimal.json"),
    ),
    (
        "credential-full.json",
        include_str!("jose/fixtures/w3c-vc-jose-cose/credential-full.json"),
    ),
    (
        "presentation-single.json",
        include_str!("jose/fixtures/w3c-vc-jose-cose/presentation-single.json"),
    ),
    (
        "presentation-multiple.json",
        include_str!("jose/fixtures/w3c-vc-jose-cose/presentation-multiple.json"),
    ),
    (
        "credential-unknown-extensions.json",
        include_str!("jose/fixtures/w3c-vc-jose-cose/credential-unknown-extensions.json"),
    ),
    (
        "credential-jose-minimal.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/credential-jose-minimal.txt"),
    ),
    (
        "presentation-jose-multiple.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/presentation-jose-multiple.txt"),
    ),
    (
        "credential-issuer-match-signed.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/credential-issuer-match-signed.txt"),
    ),
    (
        "credential-jose-unknown-extensions.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/credential-jose-unknown-extensions.txt"),
    ),
    (
        "credential-jose-bad-signature.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/credential-jose-bad-signature.txt"),
    ),
    (
        "credential-jose-bad-media-type.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/credential-jose-bad-media-type.txt"),
    ),
    (
        "presentation-jose-bad-media-type.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/presentation-jose-bad-media-type.txt"),
    ),
    (
        "credential-jose-vc-vp-claims.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/credential-jose-vc-vp-claims.txt"),
    ),
    (
        "presentation-jose-bad-credential.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/presentation-jose-bad-credential.txt"),
    ),
    (
        "vm-ed25519.json",
        include_str!("jose/fixtures/w3c-vc-jose-cose/vm-ed25519.json"),
    ),
    (
        "vm-p256.json",
        include_str!("jose/fixtures/w3c-vc-jose-cose/vm-p256.json"),
    ),
    (
        "vm-p384.json",
        include_str!("jose/fixtures/w3c-vc-jose-cose/vm-p384.json"),
    ),
    (
        "vm-p521.json",
        include_str!("jose/fixtures/w3c-vc-jose-cose/vm-p521.json"),
    ),
    (
        "credential-selective.json",
        include_str!("jose/fixtures/w3c-vc-jose-cose/credential-selective.json"),
    ),
    (
        "credential-nested-selective.json",
        include_str!("jose/fixtures/w3c-vc-jose-cose/credential-nested-selective.json"),
    ),
    (
        "presentation-selective.json",
        include_str!("jose/fixtures/w3c-vc-jose-cose/presentation-selective.json"),
    ),
    (
        "credential-sdjwt-selective.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/credential-sdjwt-selective.txt"),
    ),
    (
        "credential-sdjwt-nested.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/credential-sdjwt-nested.txt"),
    ),
    (
        "presentation-sdjwt-selective.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/presentation-sdjwt-selective.txt"),
    ),
    (
        "credential-sdjwt-bad-signature.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/credential-sdjwt-bad-signature.txt"),
    ),
    (
        "credential-sdjwt-bad-media-type.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/credential-sdjwt-bad-media-type.txt"),
    ),
    (
        "presentation-sdjwt-bad-media-type.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/presentation-sdjwt-bad-media-type.txt"),
    ),
    (
        "presentation-sdjwt-bad-credential.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/presentation-sdjwt-bad-credential.txt"),
    ),
    (
        "credential-cose-minimal.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/credential-cose-minimal.txt"),
    ),
    (
        "credential-cose-minimal-not-base64.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/credential-cose-minimal-not-base64.txt"),
    ),
    (
        "credential-cose-bad-signature.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/credential-cose-bad-signature.txt"),
    ),
    (
        "credential-cose-bad-media-type.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/credential-cose-bad-media-type.txt"),
    ),
    (
        "presentation-cose-single.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/presentation-cose-single.txt"),
    ),
    (
        "presentation-cose-bad-credential.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/presentation-cose-bad-credential.txt"),
    ),
    (
        "presentation-cose-bad-media-type.txt",
        include_str!("jose/fixtures/w3c-vc-jose-cose/presentation-cose-bad-media-type.txt"),
    ),
    (
        "vm-did-ed25519.json",
        include_str!("jose/fixtures/w3c-vc-jose-cose/vm-did-ed25519.json"),
    ),
    (
        "LICENSE.md",
        include_str!("jose/fixtures/w3c-vc-jose-cose/LICENSE.md"),
    ),
];

/// The text of the vendored file `name`.
pub(crate) fn file(name: &str) -> &'static str {
    FILES
        .iter()
        .find(|(file, _)| *file == name)
        .unwrap_or_else(|| panic!("{name} is not a vendored file"))
        .1
}

/// A vendored JSON file, parsed.
pub(crate) fn json(name: &str) -> Value {
    serde_json::from_str(file(name)).unwrap_or_else(|e| panic!("{name} is not JSON: {e}"))
}

/// A vendored token. A file may end in a newline, which is no part of the token.
pub(crate) fn token(name: &str) -> String {
    file(name).trim_end().to_owned()
}

/// A vendored COSE input read as the base64 the suite writes: the standard alphabet with no
/// padding, in a file that may end in a newline. Decoded with the strict engine of the
/// `base64` crate, so a file that is not that shape fails here and not in the test.
pub(crate) fn cose_input(name: &str) -> Vec<u8> {
    base64::engine::general_purpose::STANDARD_NO_PAD
        .decode(file(name).trim_end())
        .unwrap_or_else(|e| panic!("{name} is not unpadded base64: {e}"))
}

/// The key file of `algorithm`: `p256`, `p384`, `p521`, `ed25519` or `did-ed25519`.
pub(crate) fn key_file(algorithm: &str) -> Value {
    json(&format!("vm-{algorithm}.json"))
}

/// The public key of the key file of `algorithm`.
pub(crate) fn public_key(algorithm: &str) -> PublicKey {
    PublicKey::from_jwk(&key_file(algorithm)["publicKeyJwk"]).expect("a public key")
}

/// The signing key of the key file of `algorithm`.
pub(crate) fn signing_key(algorithm: &str) -> KeyEntry {
    KeyEntry::from_private_jwk(&key_file(algorithm)["secretKeyJwk"]).expect("a private key")
}

/// The `id` of the key file of `algorithm`: the absolute URL of its verification
/// method, which is the `kid` the suite's issuer would sign under.
pub(crate) fn key_id(algorithm: &str) -> String {
    key_file(algorithm)["id"]
        .as_str()
        .expect("an id")
        .to_owned()
}

/// The `kid` the tests sign under: an absolute URL.
pub(crate) const KID: &str = "https://issuer.example/keys#1";

/// A key this test generated, of `algorithm`.
pub(crate) fn generated(algorithm: KeyAlgorithm) -> KeyEntry {
    temp_store_with("jose", "k", algorithm)
        .load_key("k")
        .expect("the key was just generated")
}

/// A credential that uses only base-vocabulary terms.
pub(crate) fn credential() -> Value {
    json!({
        "@context": ["https://www.w3.org/ns/credentials/v2"],
        "id": "urn:uuid:7b0a4c1e-3f5d-4c8e-9a41-2d6f8e0b1c33",
        "type": ["VerifiableCredential"],
        "issuer": "https://issuer.example",
        "validFrom": "2025-01-01T00:00:00Z",
        "credentialSubject": {"id": "did:example:subject", "name": "Example"},
    })
}

/// The protected header `sign_credential` writes for `key`.
pub(crate) fn credential_header(key: &KeyEntry) -> Value {
    json!({"alg": key.algorithm().jose_alg(), "kid": KID, "typ": "vc+jwt", "cty": "vc"})
}

/// The protected header `sign_presentation` writes for `key`.
pub(crate) fn presentation_header(key: &KeyEntry) -> Value {
    json!({"alg": key.algorithm().jose_alg(), "kid": KID, "typ": "vp+jwt", "cty": "vp"})
}

/// A compact JWS with a genuine signature by `key` over `header` and `payload`.
pub(crate) fn forge(key: &KeyEntry, header: &Value, payload: &Value) -> String {
    let payload = serde_json::to_vec(payload).expect("serialises");
    forge_raw(
        key,
        header,
        &base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload),
    )
}

/// [`forge`] with the payload segment given as text, for a payload that is not a
/// JSON object.
pub(crate) fn forge_raw(key: &KeyEntry, header: &Value, payload_segment: &str) -> String {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header = b64.encode(serde_json::to_vec(header).expect("serialises"));
    let input = format!("{header}.{payload_segment}");
    format!(
        "{input}.{}",
        b64.encode(key.sign(input.as_bytes()).expect("an unrevoked key signs"))
    )
}

/// An Enveloped Verifiable Credential holding `message` as a `vc+cose`, written as this crate
/// writes a `data:` URL: padded standard base64 with no whitespace (RFC 2397, RFC 2045).
pub(crate) fn cose_envelope(message: &[u8]) -> Value {
    json!({
        "@context": "https://www.w3.org/ns/credentials/v2",
        "type": "EnvelopedVerifiableCredential",
        "id": format!(
            "data:application/vc+cose;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(message)
        ),
    })
}

/// An Enveloped Verifiable Credential holding `token` as a `vc+jwt`.
pub(crate) fn envelope(token: &str) -> Value {
    json!({
        "@context": "https://www.w3.org/ns/credentials/v2",
        "type": "EnvelopedVerifiableCredential",
        "id": format!("data:application/vc+jwt,{token}"),
    })
}

/// An Enveloped Verifiable Credential of another media type, with data this crate
/// never reads.
pub(crate) fn foreign_envelope(media_type: &str) -> Value {
    json!({
        "@context": "https://www.w3.org/ns/credentials/v2",
        "type": "EnvelopedVerifiableCredential",
        "id": format!("data:{media_type},opaque"),
    })
}

/// A presentation of `envelopes`, using only base-vocabulary terms.
pub(crate) fn presentation(envelopes: Vec<Value>) -> Value {
    json!({
        "@context": ["https://www.w3.org/ns/credentials/v2"],
        "id": "urn:uuid:0d9c6b0e-5a3c-4f43-8f0e-6f2a8d1c9e55",
        "type": ["VerifiablePresentation"],
        "holder": "https://holder.example",
        "verifiableCredential": envelopes,
    })
}

/// A forged credential token whose protected header `change` has altered.
pub(crate) fn with_header(key: &KeyEntry, change: impl FnOnce(&mut Value)) -> String {
    let mut header = credential_header(key);
    change(&mut header);
    forge(key, &header, &credential())
}

/// A forged credential token whose payload `change` has altered.
pub(crate) fn with_credential(key: &KeyEntry, change: impl FnOnce(&mut Value)) -> String {
    let mut document = credential();
    change(&mut document);
    forge(key, &credential_header(key), &document)
}

// ── SD-JWT forms ──────────────────────────────────────────────────────────────

/// The protected header `sign_credential_sd_jwt` writes for `key`.
pub(crate) fn sd_credential_header(key: &KeyEntry) -> Value {
    json!({"alg": key.algorithm().jose_alg(), "kid": KID, "typ": "vc+sd-jwt", "cty": "vc"})
}

/// The protected header `sign_presentation_sd_jwt` writes for `key`.
pub(crate) fn sd_presentation_header(key: &KeyEntry) -> Value {
    json!({"alg": key.algorithm().jose_alg(), "kid": KID, "typ": "vp+sd-jwt", "cty": "vp"})
}

/// An SD-JWT whose Issuer-signed JWT carries a genuine signature by `key` over `header` and
/// `payload`, with `disclosures` appended as they are, matched against the payload or not.
pub(crate) fn forge_sd(
    key: &KeyEntry,
    header: &Value,
    payload: &Value,
    disclosures: &[dpp_crypto::sd_jwt::Disclosure],
) -> String {
    dpp_crypto::sd_jwt::SdJwt::new(forge(key, header, payload), disclosures.to_vec()).serialise()
}

/// An SD-JWT over `document` in which each top-level member named in `conceal` is a
/// Disclosure, signed by `key` under `header`. Every Disclosure travels.
pub(crate) fn forge_sd_concealing(
    key: &KeyEntry,
    header: &Value,
    document: &Value,
    conceal: &[&str],
) -> String {
    let (kept, disclosures) =
        dpp_crypto::sd_jwt::conceal(document.as_object().expect("an object"), |name| {
            conceal.contains(&name)
        })
        .expect("conceals");
    let payload = dpp_crypto::sd_jwt::build_payload(kept, !disclosures.is_empty());
    forge_sd(key, header, &payload, &disclosures)
}

/// An Enveloped Verifiable Credential holding `sd_jwt` as a `vc+sd-jwt`.
pub(crate) fn sd_envelope(sd_jwt: &str) -> Value {
    json!({
        "@context": "https://www.w3.org/ns/credentials/v2",
        "type": "EnvelopedVerifiableCredential",
        "id": format!("data:application/vc+sd-jwt,{sd_jwt}"),
    })
}
