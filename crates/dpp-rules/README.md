# dpp-rules

[![crates.io](https://img.shields.io/crates/v/dpp-rules.svg)](https://crates.io/crates/dpp-rules)
[![docs.rs](https://img.shields.io/docsrs/dpp-rules)](https://docs.rs/dpp-rules)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

Pure `#![no_std]`, zero-dependency EU ESPR cross-field regulatory rules for the[Odal Node](https://odal-node.io) Digital Product Passport system.

These are rules that JSON Schema cannot express — "fibre percentages must sum to ~100%", "SVHC concentration > 0.1% triggers disclosure", "surfactant band must be one of the four EU-standard labels". They live here, **once**, and are consumed by both `dpp-domain` (standalone validation, no Wasm host) and the Wasm sector plugins (via `dpp-plugin-sdk::rules`). Every regulatory rule has exactly one implementation.

Keeping them in a standalone `no_std`, zero-dependency crate is what makes that
possible: `dpp-domain` and the Wasm plugins can both depend on it without either
depending on the other.

---

## When to use this crate

- You need a cross-field regulatory check that JSON Schema cannot express — a
  sum constraint, a conditional disclosure trigger, an enumerated band.
- You are writing a Wasm sector plugin and need the same rule the host applies,
  compiled into the guest.
- You are adding a rule and need it to exist exactly once, reachable from both
  `dpp-domain` and the plugins.

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

---

## No-std constraint

This crate **must remain `#![no_std]` + `alloc` with zero dependencies.** That is the whole reason C1 extracted it from `dpp-domain`: sector plugins compile to `wasm32-wasip1` and cannot pull in the heavier domain crate. If a rule needs `std` or an external crate, the rule is in the wrong place.

The CI `cargo check` target verifies this. Do not break it.

---

## Module structure

```text
src/
├── lib.rs                    re-exports everything at the crate root (backward compat)
│
├── common/
│   ├── country.rs            ISO 3166-1 alpha-2 validation ✅
│   ├── numeric.rs            percentage helpers, sum checks      (placeholder)
│   └── units.rs              unit conversion helpers              (placeholder)
│
├── chemicals/                REACH / RoHS / EU 2026/405  (cross-sector — never under one sector)
│   ├── svhc.rs               REACH Art. 33 concentration validation ✅
│   └── surfactants.rs        EU 2026/405 band validation ✅
│
├── textiles/                 EU ESPR textile sector
│   ├── fibre.rs              fibre_sum_ok, validate_fibre_composition ✅
│   └── care.rs               ISO 3758 care symbols                (placeholder)
│
├── batteries/                EU Regulation 2023/1542, Annex XIII
│   ├── chemistry.rs          allowed chemistries, voltage ranges  (placeholder)
│   ├── degradation.rs        SOH estimation, cycle life           (placeholder)
│   └── recycled_content.rs   Co / Li / Ni split thresholds       (placeholder)
│
├── electronics/              EU Ecodesign Regulation (ESPR)
│   └── spare_parts.rs        availability periods                 (placeholder)
│                             NOTE: repairability scoring → dpp-calc, not here
│
├── metals/                   CBAM (EU Regulation 2023/956)
│   ├── aluminium.rs          alloy grade, CO₂e per route          (placeholder)
│   └── steel.rs              BF-BOF / EAF / DRI-EAF, scrap ratio (placeholder)
│
├── construction/             EU CPR 2024/3110                     (placeholder)
└── toys/                     EN 71 / REACH / EU 2025/2509         (placeholder)
```

Active sectors are batteries, textiles, and electronics. All others have placeholder modules with field documentation; rules will be added as the delegated acts finalise and the sector plugins mature.

---

## Adding a rule

1. Find the right module (sector + sub-concern). If the rule applies to more than one sector, it belongs in `chemicals/` or `common/`, not under any single sector.
2. Write a pure function with primitive borrowing inputs (`&str`, `f64`, `&[T]`). No owned allocations in function arguments.
3. Return `Result<(), String>` for validators, `bool` for predicates.
4. Add a `#[cfg(test)] mod tests` block in the same file.
5. If the symbol needs to be accessible from `dpp-domain` or `dpp-plugin-sdk` callers without the module path, add a `pub use` line in `lib.rs`.
6. Run `cargo test -p dpp-rules` and then `cargo test --workspace` before committing.

---

## Invariants

| Rule | Rationale |
|---|---|
| `#![no_std]` + `alloc` only | Wasm sector plugins consume this crate; std is unavailable in `wasm32-wasip1` |
| Zero `[dependencies]` in `Cargo.toml` | Keeps the plugin graph lean; any dep here becomes a dep of every plugin |
| Primitive borrowing inputs | Callers adapt their own types; this crate depends on neither `dpp-domain` structs nor `serde_json::Value` |
| One implementation per rule | The only acceptable duplication is a `pub use` re-export in `lib.rs` |
| Repairability scoring → `dpp-calc` | EN 45554 A–E grade calculation belongs to the calculator crate; only field-level validations live here |
| SVHC → `chemicals/` | REACH Art. 33 is cross-sector; it must never be placed under a single sector module |
| `common/` threshold | A helper belongs in `common/` only if it is used by ≥ 2 sector modules and has no sector-specific meaning |

---

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-domain` | Consumes these rules for standalone validation; this crate does **not** depend on it |
| `dpp-plugin-sdk` | Re-exports these rules to Wasm sector plugins via `dpp_plugin_sdk::rules` |
| `dpp-calc` | Holds scoring and calculation; field-level validation belongs here, not there |

## Minimum Rust version

1.96 (MSRV is enforced in CI)

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
