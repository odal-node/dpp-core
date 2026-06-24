# dpp-registry

EU EUDPP Central Registry connector for the [Odal Node](https://odal-node.io)
Digital Product Passport system.

Currently a **ghost connector**: the EU Central Registry API has not been published
yet. This crate defines all interface types (registration payloads, sync requests,
response envelopes, error models) so that platform code can be written against a
stable interface today. The real HTTP adapter lives in `dpp-engine` and will be
wired in once the EU endpoint is live.

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
