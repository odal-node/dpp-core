# dpp-rules

[![crates.io](https://img.shields.io/crates/v/dpp-rules.svg)](https://crates.io/crates/dpp-rules)
[![docs.rs](https://img.shields.io/docsrs/dpp-rules)](https://docs.rs/dpp-rules)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

Cross-field regulatory rules for EU Digital Product Passports, as pure `#![no_std]`
functions with no dependencies by default. Part of
[dpp-core](https://github.com/odal-node/dpp-core) by [Odal Node](https://odal-node.io).

These are the rules JSON Schema cannot express: fibre percentages summing to about
100%, an SVHC concentration above 0.1% triggering disclosure, a surfactant band being
one of the four EU labels. Each rule is implemented **once** here and used by both
`dpp-domain` (validation without a Wasm host) and the Wasm product group plugins
(through `dpp-plugin-sdk::rules`). Because this crate is `no_std` with no dependencies,
both can depend on it without depending on each other.

## When to use this crate

- You need a cross-field regulatory check that JSON Schema cannot express: a sum
  constraint, a conditional disclosure trigger, an enumerated band.
- You write a Wasm product group plugin and need the same rule `dpp-domain` applies,
  compiled into the guest.
- You add a rule and need it to exist exactly once, reachable from `dpp-domain` and
  from the plugins.

## Example

```rust
use dpp_rules::{FibreInput, fibre_sum_ok, validate_fibre_composition};

// Fibre percentages must sum to ~100% (± FIBRE_SUM_TOLERANCE)
assert!(fibre_sum_ok(&[70.0, 30.0]));
assert!(!fibre_sum_ok(&[70.0, 20.0]));

// The full cross-field rule also checks ranges and ISO 3166-1 alpha-2 origins
let fibres = [
    FibreInput { fibre: "organic cotton", pct: 70.0, country_of_origin: Some("IN") },
    FibreInput { fibre: "recycled polyester", pct: 30.0, country_of_origin: None },
];
assert!(validate_fibre_composition(&fibres).is_ok());
```

## `no_std` and dependencies

The default build is `#![no_std]` plus `alloc`, with no dependencies. Product group
plugins compile to `wasm32-wasip1` and must not pull in the heavier domain crate, which
is why these rules live in their own crate. If a rule needs `std` or an external crate,
it belongs somewhere else. CI checks this.

The one exception is the opt-in `bundle` feature: the signed ruleset-bundle format and
its verification. It enables `std` and brings in `serde`, `serde_json`, `serde_jcs`,
`chrono`, `sha2`, `base64`, `hex` and `thiserror`. Plugins never enable it.

## Modules

| Module | Covers |
|---|---|
| `common` | Country codes, dates, numeric helpers, and the single home of identifier syntax (GTIN check digits, GS1 CSET 82) |
| `chemicals` | REACH Art. 33 SVHC concentrations, CAS numbers, Regulation (EU) 2026/405 surfactant bands. Cross-product-group, so never under one group |
| `batteries` | Regulation (EU) 2023/1542: chemistry, degradation, recycled content, passport scope and per-category mandatory content, due-diligence scope, schedules |
| `textiles` | Fibre composition |
| `electronics` | Repairability-index scope |
| `metals` | Aluminium and steel production routes and CBAM-related fields |
| `unsold_goods` | The ESPR Art. 25 destruction ban (Annex VII) and the Art. 24 disclosure |
| `lineage` | Checks that bind one passport to another: consistency, status defects, substantiation |
| `lint` | Non-binding plausibility findings for battery, textile, SVHC and unsold-goods data |
| `canonical` | RFC 8785 (JCS) content hashing, the single content hasher (feature `bundle`) |
| `bundle` | Signed, versioned ruleset bundles (feature `bundle`) |
| `construction`, `toys` | Module scaffolding with field documentation; no rules yet |

A few files are scaffolding without rules yet (`common::units`, `textiles::care`,
`electronics::spare_parts`). Rules are added as each delegated act is adopted.
[docs.rs](https://docs.rs/dpp-rules) lists every public function.

## Adding a rule

1. Pick the module by product group and concern. A rule that applies to more than one
   product group goes in `chemicals` or `common`.
2. Write a pure function with borrowed primitive inputs (`&str`, `f64`, `&[T]`), no
   owned allocations in its arguments.
3. Return `Result<(), String>` from a validator and `bool` from a predicate.
4. Add tests in a sibling test file.
5. If callers in `dpp-domain` or `dpp-plugin-sdk` need it without the module path, add a
   `pub use` in `lib.rs`.
6. Run `cargo test -p dpp-rules`, then `just check`.

## Invariants

| Rule | Why |
|---|---|
| `#![no_std]` + `alloc` in the default build | Wasm plugins use this crate, and `std` is unavailable to them |
| No non-optional `[dependencies]` | Any dependency here becomes a dependency of every plugin |
| Borrowed primitive inputs | Callers adapt their own types; this crate depends on neither `dpp-domain` structs nor `serde_json::Value` |
| One implementation per rule | The only acceptable duplication is a `pub use` re-export in `lib.rs` |
| Repairability scoring lives in `dpp-calc` | EN 45554 grade calculation is a calculator; only field-level validation lives here |
| SVHC lives in `chemicals` | REACH Art. 33 applies across product groups |
| `common` is for shared helpers | A helper goes there only if at least two product group modules use it and it has no group-specific meaning |

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-domain` | Uses these rules for validation; this crate does **not** depend on it |
| `dpp-plugin-sdk` | Re-exports these rules to plugins as `dpp_plugin_sdk::rules` |
| `dpp-calc` | Holds scoring and calculation; field-level validation stays here |

## Minimum Rust version

1.96, enforced in CI.

## Licence

Apache-2.0. See [LICENSE](../../LICENSE).
