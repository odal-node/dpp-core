# dpp-domain

[![crates.io](https://img.shields.io/crates/v/dpp-domain.svg)](https://crates.io/crates/dpp-domain)
[![docs.rs](https://img.shields.io/docsrs/dpp-domain)](https://docs.rs/dpp-domain)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

Core domain types, port traits, and schema validation for the
[Odal Node](https://odal-node.io) Digital Product Passport system.

This is the foundational crate. All other `dpp-*` crates depend on it.
It contains everything that changes when EU regulations change — and nothing else.

## When to use this crate

- You need the DPP data model: `Passport`, `SectorData`, `TransferChain`.
- You are implementing a platform adapter (database, HTTP layer) and need the
  port trait interfaces: `PassportRepository`, `IdentityPort`, `PluginHost`, etc.
- You want to validate passport data against embedded JSON schemas.

## Example

```rust
use dpp_domain::{Passport, PassportId, Sector, SectorData, TextileData, FibreEntry};

let passport = Passport::new(
    PassportId::new(),
    "did:web:manufacturer.example.com".into(),
    Sector::Textile,
    SectorData::Textile(TextileData {
        fibre_composition: vec![FibreEntry {
            fibre_type: "Organic Cotton".into(),
            percentage: 100.0,
            country_of_origin: Some("TR".into()),
        }],
        care_instructions: "Machine wash 30°C".into(),
        country_of_manufacture: "TR".into(),
        ..Default::default()
    }),
);

passport.validate().unwrap();
```

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-crypto` | Cryptographic signing, VCs, access-tier policy — depends on this crate |
| `dpp-digital-link` | GS1 Digital Link parsing, AAS mapping — depends on this crate |
| `dpp-registry` | EU Central Registry connector — depends on this crate |
| `dpp-plugin-traits` | Wasm plugin ABI — standalone, no dependency on this crate |

## Minimum Rust version

1.96 (MSRV is enforced in CI)

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
