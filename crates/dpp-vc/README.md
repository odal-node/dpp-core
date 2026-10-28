# dpp-vc

[![crates.io](https://img.shields.io/crates/v/dpp-vc.svg)](https://crates.io/crates/dpp-vc)
[![docs.rs](https://img.shields.io/docsrs/dpp-vc)](https://docs.rs/dpp-vc)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

W3C Verifiable Credentials, `did:web` documents, Bitstring Status List revocation, and
the JSON-LD context they are expressed in, for EU Digital Product Passports. Part of
[dpp-core](https://github.com/odal-node/dpp-core) by [Odal Node](https://odal-node.io).

Pure Rust, no I/O, no network calls. It does not build for `wasm32-unknown-unknown`,
because it depends on `dpp-crypto`, whose RNG needs a platform entropy source.

## When to use this crate

- You issue or verify a credential that proves a supply-chain actor's role: an
  authorised repairer, a recycler, a market-surveillance authority.
- You publish or resolve a `did:web` document for a manufacturer or operator.
- You check whether a credential has been revoked against a Bitstring Status List.
- You build the JSON-LD envelope for a passport or a credential.
- You read a *copy* of a passport's public view and need to know whether its time
  bound still holds. See `snapshot`.
- You issue a passport as an **SD-JWT VC**, so a holder can present part of it to a
  third party without the issuing node being reachable. See `sd_jwt_vc`. Keys there are
  published through JWT VC Issuer Metadata rather than `did:web`, because the credential
  profile defines no DID mechanism.

  This crate does not *resolve* those keys, and nothing here does I/O.
  `build_issuer_metadata` produces the document an issuer publishes, and `verify` takes
  the key the caller has already chosen. Fetching the metadata, deciding whether to
  trust it, and picking the key are your application's job, and so is the network policy.

## What this crate does not cover

| Concern | Where it lives | Why not here |
|---|---|---|
| Cryptographic primitives: JWS, keys, keystore | `dpp-crypto` | Signing bytes is a different job from deciding whose signature means what |
| The disclosure contract: `Audience`, `Disclosure`, the per-field map and its filter | `dpp-domain` | Which fields a role may see describes what a passport *is*, not who is asking |
| Projections: GS1, AAS | `dpp-digital-link`, `dpp-aas` | Rendering a passport for an ecosystem says nothing about trust |

The boundary: **a credential establishes which `Audience` a caller holds; the
disclosure policy maps that `Audience` to fields.** Two questions, two places.

## Example

```rust
use dpp_vc::{CredentialBuilder, CredentialRole, DppCredentialSubject};

let subject = DppCredentialSubject {
    id: "did:web:repairer.example.com".into(),
    name: "Example Repair GmbH".into(),
    role: CredentialRole::AuthorisedRepairer,
    country: "DE".into(),
    product_groups: vec!["battery".into()],
    product_categories: vec![],
};

let credential = CredentialBuilder::new(
    "did:web:issuer.example.com".into(),
    subject,
)
.expires_in_days(365)
.build();
```

A runnable version covering issuance and transfer is in
[`examples/credential_and_transfer.rs`](examples/credential_and_transfer.rs).

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-domain` | Provides `Audience`, `Disclosure`, `PassportId` and `IdentityPort`; required |
| `dpp-crypto` | Provides JWS signing and verification and the keystore; required |

The HTTP surface that issues and verifies these credentials is not part of this
workspace; it belongs to your application. Everything below that
boundary (credential shapes, signing, status lists) is here and works without one.

## Minimum Rust version

1.96, enforced in CI.

## Licence

Apache-2.0. See [LICENSE](../../LICENSE).
