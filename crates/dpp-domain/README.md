# dpp-domain

[![crates.io](https://img.shields.io/crates/v/dpp-domain.svg)](https://crates.io/crates/dpp-domain)
[![docs.rs](https://img.shields.io/docsrs/dpp-domain)](https://docs.rs/dpp-domain)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

Core domain types, port traits, and schema validation for the
[Odal Node](https://odal-node.io) Digital Product Passport system.

This is the foundational crate: any other `dpp-*` crate may depend on it, and
several do. It contains everything that changes when EU regulations change — and
nothing else.

## When to use this crate

- You need the DPP data model: `Passport`, `ProductGroupData`, `TransferChain`.
- You need to know **what law reaches a product group**: `InstrumentCatalog`
  holds one manifest per act, with a `PassportObligation` and one
  `InstrumentBinding` per (act, product group) pair. Obligations accumulate —
  ESPR Art. 5(7) lets acts overlap and sets no precedence rule between them — so
  this answers with a *set*, and a determination is always made under a named act.
- You are implementing a platform adapter (database, HTTP layer) and need the
  port trait interfaces: `PassportRepository`, `IdentityPort`, `PluginHost`, etc.
- You want to validate passport data against embedded JSON schemas.

## Example

```rust
use dpp_domain::access::{ProductGroupAccessPolicy, filter_by_audience};
use dpp_domain::catalog::ProductGroupCatalog;
use dpp_domain::Audience;
use serde_json::json;

// Product groups are data, not code — one embedded manifest each. The descriptor
// carries identity, scope, schema versions, disclosure and plugin binding, and no
// law at all: status, legal basis, passport obligation, dates, retention and
// granularity are properties of an (act, product group) pair and live on
// `InstrumentBinding` in `catalog::InstrumentCatalog`.
let catalog = ProductGroupCatalog::new();
let battery = catalog.get("battery").expect("battery is in the catalog");
assert_eq!(battery.key, "battery");

// Disclosure classes come from the schema *version* a passport declares,
// not from one unversioned map. That is what lets a published passport keep
// the classification its signatures were taken under: reclassifying a field
// is a new schema version, and an older passport goes on being filtered by
// the version it was validated against.
let policy = ProductGroupAccessPolicy::for_schema_version("battery", &battery.current_schema_version)
    .expect("the current battery schema classifies every property");

// A passport: envelope fields, with the product group's own data nested under
// `productGroupData`. The nesting is not decoration — a product group's schema
// classifies its own payload and has no authority over the envelope around it.
let full = json!({
    "productName": "EcoCell",
    "productGroupData": { "productGroup": "battery", "stateOfHealthPct": 87.5 }
});
let public = filter_by_audience(&full, &policy, Audience::Public);

// State of health is Annex XIII point 4 — data about one individual battery,
// reserved to holders of a legitimate interest and withheld from the public.
assert_eq!(public.filtered_data["productName"], "EcoCell");
assert!(public.filtered_data["productGroupData"].get("stateOfHealthPct").is_none());
```

To filter a bare product-group payload — a document already inside
`productGroupData`, which is how a two-pass caller handles the two halves
separately — use `filter_by_audience_in_scope` and pass
`DocumentScope::ProductGroupData`. Filtering a payload as an envelope applies
none of the product group's classes, so every restricted field in it would be
served.

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
