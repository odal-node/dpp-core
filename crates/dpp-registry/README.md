# dpp-registry

[![crates.io](https://img.shields.io/crates/v/dpp-registry.svg)](https://crates.io/crates/dpp-registry)
[![docs.rs](https://img.shields.io/docsrs/dpp-registry)](https://docs.rs/dpp-registry)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

EU EUDPP Central Registry connector for the [Odal Node](https://odal-node.io)
Digital Product Passport system.

Currently a **ghost connector**. This crate defines interface types (registration
payloads, sync requests, response envelopes, error models) so that platform code
can be written against a stable interface. The real HTTP adapter lives in
`dpp-engine`.

⚠️ **These shapes predate the published specification.** The registry became
operational on 20 July 2026 under Commission Implementing Regulation (EU)
2026/1778; the types here were derived before it and are known to diverge — see
the EU Registry Readiness section of `docs/regulatory/COMPLIANCE.md`.
Reconciliation is a breaking change scheduled for the next minor. Do not treat
these as an implementation target.

`wasm32-unknown-unknown` safe — no I/O, no `std` networking.

## When to use this crate

- You are implementing `RegistrySyncPort` in a platform adapter and need the
  request/response types.
- You want to model passport registration and transfer-notification payloads
  for the EU Central Registry.

## Example

```rust
use dpp_registry::registry::{RegistrationPayload, RegistryEnvelope, RegistryStatus};

let payload = RegistrationPayload {
    passport_id: "urn:dpp:passport:abc123".into(),
    issuer_did: "did:web:manufacturer.example.com".into(),
    sector: "textile".into(),
    schema_version: "1.1.0".into(),
};

// Wrap in the standard envelope before sending to the EU endpoint
let envelope = RegistryEnvelope::new(payload);
```

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-domain` | Provides `Passport`, `PassportId`, `DppError` — required by this crate |

## Minimum Rust version

1.96 (MSRV is enforced in CI)

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
