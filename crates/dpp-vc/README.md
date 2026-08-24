# dpp-vc

[![crates.io](https://img.shields.io/crates/v/dpp-vc.svg)](https://crates.io/crates/dpp-vc)
[![docs.rs](https://img.shields.io/docsrs/dpp-vc)](https://docs.rs/dpp-vc)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

The trust layer for the [Odal Node](https://odal-node.io) Digital Product
Passport system: W3C Verifiable Credentials, `did:web` documents, Bitstring
Status List revocation, and the JSON-LD context those are expressed in.

Pure Rust, no I/O, no network calls. Not a `wasm32-unknown-unknown` target — it
depends on `dpp-crypto`, whose RNG requires a platform entropy source.

## When to use this crate

- You are issuing or verifying a credential that proves a supply-chain actor's
  role — an authorised repairer, a recycler, a market-surveillance authority.
- You need to publish or resolve a `did:web` document for a manufacturer or
  operator.
- You need to check whether a credential has been revoked against a Bitstring
  Status List.
- You are building the JSON-LD envelope for a passport or a credential.

## What this crate is *not*

Three neighbouring concerns live elsewhere, and the boundaries are deliberate:

| Concern | Home | Why not here |
|---|---|---|
| Cryptographic primitives — JWS, keys, keystore | `dpp-crypto` | Signing bytes is a different job from deciding whose signature means what |
| The disclosure contract — `Audience`, `Disclosure`, the per-field map and its filter | `dpp-crypto::access` today; `dpp-domain` once that crate is analysed | Which fields a role may see describes what a passport *is*, not who is asking |
| Projections — GS1, AAS | `dpp-digital-link`, `dpp-aas` | Rendering a passport for an ecosystem says nothing about trust |

The seam this crate sits on: **a credential establishes which `Audience` a
caller holds; the disclosure policy maps that `Audience` to fields.** Two
questions, two places.

## Example

```rust
use dpp_vc::{CredentialBuilder, CredentialRole, DppCredentialSubject};

let subject = DppCredentialSubject {
    id: "did:web:repairer.greenfix.de".into(),
    name: "GreenFix Repair GmbH".into(),
    role: CredentialRole::AuthorisedRepairer,
    country: "DE".into(),
    product_groups: vec!["textile".into()],
    product_categories: vec![],
};

let credential = CredentialBuilder::new(
    "did:web:authority.trade-registry.europa.eu".into(),
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
| `dpp-domain` | Provides `Audience`, `Disclosure`, `PassportId` and `IdentityPort` — required by this crate |
| `dpp-crypto` | Provides JWS signing/verification and the keystore — required by this crate |
| `dpp-engine` (BSL-1.1) | Implements the HTTP surface that issues and verifies these credentials |

## Minimum Rust version

1.96 (MSRV is enforced in CI)

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
