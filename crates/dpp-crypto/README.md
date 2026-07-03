# dpp-crypto

[![crates.io](https://img.shields.io/crates/v/dpp-crypto.svg)](https://crates.io/crates/dpp-crypto)
[![docs.rs](https://img.shields.io/docsrs/dpp-crypto)](https://docs.rs/dpp-crypto)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

Cryptographic primitives for the [Odal Node](https://odal-node.io) Digital Product
Passport system: Ed25519 key management, JWS signing and verification, AES-256-GCM
field-level encryption, `did:web` document builder, W3C Verifiable Credentials, and
the ESPR Art. 10 access-tier policy engine.

## When to use this crate

- You need to sign or verify DPP data with Ed25519.
- You are issuing or verifying W3C Verifiable Credentials for supply-chain actors.
- You need to enforce read-access tiers (Public / Professional / Confidential) on
  passport fields per ESPR Article 10.
- You are building a `did:web` identity for a manufacturer or operator.

## Example

```rust
use dpp_crypto::credential::{CredentialBuilder, CredentialRole, DppCredentialSubject};
use chrono::Utc;

let subject = DppCredentialSubject {
    id: "did:web:repairer.greenfix.de".into(),
    name: "GreenFix Repair GmbH".into(),
    role: CredentialRole::AuthorisedRepairer,
    country: "DE".into(),
    sectors: vec!["textile".into()],
    product_categories: vec![],
};

let credential = CredentialBuilder::new(
    "did:web:authority.trade-registry.europa.eu".into(),
    subject,
)
.expires_in_days(365)
.build();
```

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-domain` | Provides `AccessTier` and `IdentityPort` — required by this crate |

## Minimum Rust version

1.96 (MSRV is enforced in CI)

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
