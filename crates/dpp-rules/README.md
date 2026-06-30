# dpp-rules

Pure `#![no_std]`, zero-dependency EU ESPR cross-field regulatory rules.

These are rules that JSON Schema cannot express — "fibre percentages must sum to ~100%", "SVHC concentration > 0.1% triggers disclosure", "surfactant band must be one of the four EU-standard labels". They live here, **once**, and are consumed by both `dpp-domain` (standalone validation, no Wasm host) and the Wasm sector plugins (via `dpp-plugin-sdk::rules`). Every regulatory rule has exactly one implementation.

See `docs/architecture/SECTOR-MODEL-CONSOLIDATION.md` §7 for the design rationale.

---

## No-std constraint

This crate **must remain `#![no_std]` + `alloc` with zero dependencies.** That is the whole reason C1 extracted it from `dpp-domain`: sector plugins compile to `wasm32-wasip1` and cannot pull in the heavier domain crate. If a rule needs `std` or an external crate, the rule is in the wrong place.

The CI `cargo check` target verifies this. Do not break it.

---

## Module structure

```
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
