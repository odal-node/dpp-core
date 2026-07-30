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
use dpp_domain::access::{SectorAccessPolicy, filter_by_audience};
use dpp_domain::catalog::SectorCatalog;
use dpp_domain::Audience;
use serde_json::json;

// Sector metadata is data, not code: regime, status, retention and per-field
// disclosure all come from the catalog manifests.
let catalog = SectorCatalog::new();
let battery = catalog.get("battery").expect("battery is in the catalog");
assert_eq!(battery.key, "battery");

// The disclosure classes declared in that manifest are what the filter applies.
let policy = SectorAccessPolicy::from_catalog(&catalog, "battery")
    .expect("battery declares a disclosure policy");

let full = json!({ "productName": "EcoCell", "stateOfHealthPct": 87.5 });
let public = filter_by_audience(&full, &policy, Audience::Public);

// State of health is per-item data — withheld from the public audience.
assert_eq!(public.filtered_data["productName"], "EcoCell");
assert!(public.filtered_data.get("stateOfHealthPct").is_none());
```

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-crypto` | JWS signing and the keystore — no dependency in either direction |
| `dpp-vc` | Credentials, `did:web` and status lists — depends on this crate |
| `dpp-digital-link` | GS1 Digital Link parsing — depends on this crate |
| `dpp-aas` | AAS submodel mapping — depends on this crate |
| `dpp-registry` | EU Central Registry connector — depends on this crate |
| `dpp-plugin-traits` | Wasm plugin ABI — standalone, no dependency on this crate |

## Minimum Rust version

1.96 (MSRV is enforced in CI)

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
