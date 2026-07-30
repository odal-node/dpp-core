# dpp-crypto

[![crates.io](https://img.shields.io/crates/v/dpp-crypto.svg)](https://crates.io/crates/dpp-crypto)
[![docs.rs](https://img.shields.io/docsrs/dpp-crypto)](https://docs.rs/dpp-crypto)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

Cryptographic primitives for the [Odal Node](https://odal-node.io) Digital
Product Passport system: Ed25519 key management, JWS signing and verification,
JCS canonicalisation, and an AES-256-GCM encrypted keystore with Argon2id key
derivation and rotation.

Pure Rust, no I/O beyond the keystore file. Not a `wasm32-unknown-unknown`
target — the RNG requires a platform entropy source.

> **Scope note.** Verifiable Credentials, `did:web` documents and status lists
> moved to [`dpp-vc`](../dpp-vc): signing bytes is a different job from deciding
> whose signature means what. The `access` module — the per-field disclosure
> policy and its filter — remains here pending analysis of `dpp-domain`, which
> is its settled home. See `dpp-docs/decisions/ADR-010-crate-architecture.md`.

## When to use this crate

- You need to sign or verify DPP data with Ed25519 as a compact JWS.
- You need JCS canonicalisation so two nodes produce byte-identical payloads
  before signing.
- You are managing signing keys at rest — generation, rotation, revocation,
  archival — with encryption and integrity protection.
- You need to apply a sector's disclosure policy to a passport payload for a
  given audience (`access`, pending its move to `dpp-domain`).

## Example

```rust
use dpp_crypto::jws::{signer, verifier};
use dpp_crypto::keystore::KeyStore;
use serde_json::json;

fn sign_a_payload(store: &KeyStore) {
    let payload = json!({ "product": "battery", "status": "published" });
    let jws = signer::sign(store, "issuer", &payload).expect("sign");

    // The kid travels in the JWS header, so a verifier can select the right key
    // from a DID document that may list several, including rotated ones.
    let kid = verifier::extract_kid_from_jws(&jws).expect("kid present");
    assert!(!kid.is_empty());
}
```

`KeyStore::open` **creates the file if it is absent**, so the path is a real
decision — pass a configured location outside the source tree. A complete,
runnable version including keystore setup is in
[`examples/sign_and_verify.rs`](examples/sign_and_verify.rs):

```bash
cargo run -p dpp-crypto --example sign_and_verify
```

## Security notes

- The keystore is protected by Argon2id key derivation with HMAC-SHA256
  integrity over the stored records. It has **not** been externally audited.
- `KeyStore::public_key` and `PublicKeyInfo` are public so the `did:web` builder
  in `dpp-vc` can read public key material and revocation state. Neither exposes
  private key material.
- Signing uses `alg: "EdDSA"` with curve `Ed25519`, per RFC 8037.

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-domain` | Provides `Audience`, `Disclosure`, `SectorCatalog` and `IdentityPort` — required by this crate |
| `dpp-vc` | Depends on this crate for JWS and the keystore; holds the credential and DID layer |

## Minimum Rust version

1.96 (MSRV is enforced in CI)

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
