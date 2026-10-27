# dpp-crypto

[![crates.io](https://img.shields.io/crates/v/dpp-crypto.svg)](https://crates.io/crates/dpp-crypto)
[![docs.rs](https://img.shields.io/docsrs/dpp-crypto)](https://docs.rs/dpp-crypto)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

Cryptographic primitives for EU Digital Product Passports: Ed25519 key management, JWS
signing and verification, JCS canonicalisation, JAdES baseline signature construction,
and an AES-256-GCM encrypted keystore with Argon2id key derivation and rotation. Part of
[dpp-core](https://github.com/odal-node/dpp-core) by [Odal Node](https://odal-node.io).

Pure Rust, with no I/O beyond the keystore file. It does not build for
`wasm32-unknown-unknown`, because the RNG needs a platform entropy source.

**This crate has no workspace dependencies.** Verifiable Credentials, `did:web`
documents and status lists are in [`dpp-vc`](../dpp-vc); the per-field disclosure
policy is `dpp_domain::access`. What remains here is primitives, and none of them needs
a domain type.

## When to use this crate

- You need to sign or verify passport data with Ed25519 as a compact JWS.
- You need JCS canonicalisation, so two nodes produce byte-identical payloads before
  signing.
- You manage signing keys at rest (generation, rotation, revocation, archival) with
  encryption and integrity protection.
- You need to build an **ETSI TS 119 182-1 JAdES-B-B** signature whose signature value
  comes from elsewhere, typically a remote signing service. The `jades` module derives
  the signing input and assembles the compact form; it never signs.
- You need **selective disclosure** over a JSON payload: claim values replaced with
  salted hashes, so a holder can reveal a subset later. The `sd_jwt` module implements
  the mechanism of IETF RFC 9901. Which claims are sensitive is `dpp-domain`'s question,
  and credentials are `dpp-vc`'s.

## Example

```rust
use dpp_crypto::jws::{signer, verifier};
use dpp_crypto::keystore::KeyStore;
use serde_json::json;

fn sign_a_payload(store: &KeyStore) {
    let payload = json!({ "product": "battery", "status": "published" });
    // The kid travels in the JWS header, so a verifier can select the right key
    // from a DID document that may list several, including rotated ones. It is
    // the key's thumbprint URI here; a token verified through a DID document
    // carries the DID URL of the verification method that ends in it.
    let public = store.public_key("issuer").expect("the key exists");
    let kid = public.thumbprint_uri().expect("a valid public key");
    let jws = signer::sign(store, "issuer", &payload, &kid).expect("sign");

    assert_eq!(verifier::extract_kid_from_jws(&jws), Some(kid));
}
```

`KeyStore::open` **creates the file if it is absent**, so choose the path deliberately
and keep it outside the source tree. A complete, runnable version including keystore
setup is in [`examples/sign_and_verify.rs`](examples/sign_and_verify.rs):

```bash
cargo run -p dpp-crypto --example sign_and_verify
```

## Security notes

- The keystore uses Argon2id key derivation and HMAC-SHA256 integrity over the stored
  records. It has **not** been externally audited.
- `KeyStore::public_key` and `PublicKeyInfo` are public so the `did:web` builder in
  `dpp-vc` can read public key material and revocation state. Neither exposes private
  key material.
- Signing uses `alg: "EdDSA"` with curve `Ed25519`, per RFC 8037.
- The `jades` module produces a **structurally conformant** JAdES-B-B signature. That
  does not make it *qualified*: under Regulation (EU) No 910/2014 a qualified signature
  needs a qualified certificate, a qualified creation device and the trust service
  provider behind them, none of which is a property of the assembled bytes. The module
  does not validate either: AdES validation is ETSI EN 319 102-1 and needs trust
  anchors, certificate paths and revocation data.

## Relationship to other crates

| Crate | Role |
|---|---|
| none | This crate depends on no other workspace crate |
| `dpp-vc` | Depends on this crate for JWS and the keystore; holds credentials, `did:web` and status lists |
| `dpp-domain` | Holds `Audience`, `Disclosure` and the disclosure filter; no dependency in either direction |

## Minimum Rust version

1.96, enforced in CI.

## Licence

Apache-2.0. See [LICENSE](../../LICENSE).
