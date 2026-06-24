# dpp-calc

Pure, stateless EU-methodology compliance calculators for [Odal Node](https://odal-node.io).

Every function in this crate is deterministic and side-effect free — no I/O, no infrastructure, no `async`.
The math is open-source (Apache-2.0). Licensed lifecycle-inventory (LCI) data is injected at runtime
through the `FactorProvider` trait and never bundled here — open methodology, licensed data supplied
by the operator. That split is the licensing rationale for this crate.

---

## When to use this crate

- You are a sector plugin (`sector-battery`, `sector-electronics`, …) and need to call an EU calculator
  to fill `PluginResult.repairability_index` or `co2e_score`.
- You are writing an integration test and need golden-vector inputs and expected outputs.
- You are adding a new EU methodology or product category (see [Adding a sector calculator](#adding-a-sector-calculator) below).

## When NOT to use this crate

- You need the DPP data model or port traits → `dpp-domain`.
- You need the Wasm plugin ABI → `dpp-plugin-traits` / `dpp-plugin-sdk`.
- You need field-level validation rules (REACH, fibre percentages, voltage ranges) → `dpp-rules`.

---

## Module structure

```
src/
├── lib.rs                    public API + sector-calculator scaling guide (read this first)
│
├── error.rs                  CalcError (InvalidInput | RulesetExpired | FactorNotFound | …)
├── receipt.rs                CalculationReceipt — proof-of-calculation envelope
├── ruleset.rs                RulesetId, RulesetVersion, EffectiveDateBound, RegulatoryBasis
│                             Ruleset trait — every methodology trait extends this
├── factor.rs                 FactorProvider trait + SyntheticFactorProvider (test/CI only)
├── ruleset_registry.rs       date-based ruleset resolution; all_rulesets() CI iterator
│
├── repairability/            EN 45554 six-parameter A–E scorer
│   ├── mod.rs                calculate(inputs, ruleset) → RepairabilityResult
│   ├── parameters.rs         RepairabilityInputs (6 × u8, ordinal 0–2)
│   ├── thresholds.rs         RepairabilityRuleset: Ruleset trait + SmartphoneTabletRuleset
│   │                         (in force June 2025) + LaptopRuleset (stub, ~2027)
│   └── golden_vectors.rs     #[cfg(test)] — JRC reference vectors, A–E coverage,
│                             regulatory-basis non-empty CI check
│
└── co2e/
    ├── mod.rs                calculate(inputs) → Co2eResult (cradle-to-gate, operator-supplied EFs)
    ├── cfb.rs                CfbRuleset: Ruleset stub + calculate_cfb() STUB (Phase 2)
    └── gwp_factors.rs        Embedded GWP100 characterisation factors (EF 3.1 / AR6 — free)
```

**Phase status:**

| Module | Status |
|---|---|
| `repairability` | ✅ In force — EU 2023/1669, smartphones/tablets, June 2025 |
| `co2e::calculate` | ✅ Baseline — operator-supplied emission factors |
| `co2e::cfb` | 🔒 Stub — gated on signed ecoinvent/EF sublicense (Phase 1 gate) |
| `pef/` (future) | 📋 Not yet — awaits per-sector PEFCR finalisation (2026–2030) |

---

## Usage

### Repairability (EN 45554 A–E)

```rust
use dpp_calc::{
    repairability::{calculate, parameters::RepairabilityInputs, SmartphoneTabletRuleset},
    ruleset_registry,
};
use chrono::Utc;

// Option A: use the ruleset directly (when you know the product category at compile time)
let result = calculate(
    &RepairabilityInputs {
        disassembly:           2,
        spare_parts:           2,
        repair_info:           1,
        diagnostic_tools:      1,
        software_updatability: 2,
        customer_support:      1,
    },
    &SmartphoneTabletRuleset,
)?;
println!("{} ({:.1}/10)", result.class, result.numeric_score); // e.g. "B (8.00/10)"
println!("receipt: {}", result.receipt.receipt_id);

// Option B: date-based resolution (when the category comes from a passport field)
let today = Utc::now().date_naive();
let ruleset = ruleset_registry::resolve_repairability("smartphone-tablet", today)
    .ok_or("no ruleset in force for this category today")?;
let result = calculate(&inputs, ruleset)?;
```

### Cradle-to-gate CO₂e

```rust
use dpp_calc::co2e::{calculate, Co2eInputs, MaterialFootprint};

let result = calculate(&Co2eInputs {
    materials: vec![
        MaterialFootprint { mass_kg: 0.5, emission_factor_kg_co2e_per_kg: 8.0 },
        MaterialFootprint { mass_kg: 0.2, emission_factor_kg_co2e_per_kg: 3.0 },
    ],
    energy_kwh: 1.5,
    grid_factor_kg_co2e_per_kwh: 0.4,
});
println!("{:.2} kg CO₂e", result.total_co2e_kg); // 5.20 kg CO₂e
```

---

## CalculationReceipt

Every calculator returns a `CalculationReceipt` alongside the computed value:

```rust
pub struct CalculationReceipt {
    pub receipt_id:             Uuid,          // UUIDv7 — time-sortable
    pub input_hash:             String,        // SHA-256 of canonical JSON inputs
    pub ruleset_id:             String,        // e.g. "smartphone-tablet-repairability"
    pub ruleset_version:        String,        // e.g. "1.0.0"
    pub factor_dataset_id:      String,        // empty if no FactorProvider used
    pub factor_dataset_version: String,
    pub factor_set_hash:        Option<String>, // SHA-256 of full factor table
    pub computed_at:            DateTime<Utc>,
}
```

The platform stores this alongside the passport record. A notified body can re-run
any calculation from the receipt: same inputs (verify via `input_hash`) + same ruleset
version + same factor dataset version → must produce the same output.

---

## RegulatoryBasis

Every concrete ruleset carries a machine-readable legal citation:

```rust
SmartphoneTabletRuleset.regulatory_basis()
// RegulatoryBasis {
//   regulation:      "EU 2023/1669",
//   article:         "Annex II, Annex III",
//   standard:        Some("EN 45554:2021"),
//   technical_study: Some("JRC128649"),
//   source_url:      Some("https://eur-lex.europa.eu/…"),
//   superseded_by:   None,
// }
```

A CI test (`expired_rulesets_have_superseded_by`) asserts that any ruleset with
`effective_dates.until < today` has a non-empty `superseded_by`. This keeps the audit chain intact as regulations evolve.

---

## Feature flags

| Flag | Default | Purpose |
|---|---|---|
| `synthetic-factors` | off | Exposes `SyntheticFactorProvider` outside `#[cfg(test)]`. Enable in integration test harnesses only. Values are **not** real LCI data. |
| `real-factors` | off | Gates licensed LCI factor-data implementations. Enable **only** in managed-service builds where a valid ecoinvent/EF reseller sublicense is in place. Never enable in open-source or self-hosted builds. |

---

## Adding a sector calculator

See the `# Adding a new sector calculator` section in `src/lib.rs` for the full
step-by-step guide. Short version:

1. **New methodology** → add `src/{methodology}/` with `mod.rs`, `parameters.rs`,
   `thresholds.rs` (trait extends `Ruleset`), `golden_vectors.rs`. Register in `ruleset_registry.rs`.
2. **New product category on an existing methodology** → add `impl Ruleset + impl {Methodology}Ruleset`
   in `thresholds.rs`, add a row to the registry, add golden vectors.
3. **Pending delegated act** → use `effective_dates.from = NaiveDate(2100, 1, 1)` as a sentinel.
4. **Superseded ruleset** → set `until`, set `superseded_by`. Never delete rows.

---

## Invariants

| Rule | Rationale |
|---|---|
| `#![forbid(unsafe_code)]` | Calculators run in the hot path; no unsafe allowed |
| Every `{Methodology}Ruleset` extends `Ruleset` | Forces a legal citation on every ruleset; compiler-enforced |
| `FactorProvider::table_hash()` is pre-computed | Called once per receipt, not per `gwp100()` lookup |
| Never bundle LCI secondary data | ecoinvent / EF inventory datasets are licensed; `FactorProvider` injects them at runtime |
| `real-factors` feature off by default | Open-source builds must never include licensed data |
| `all_rulesets()` must list every concrete ruleset | Required for the CI expiry check to cover new additions |

---

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-domain` | Domain types and port traits; `dpp-calc` does not depend on it |
| `dpp-rules` | Field-level validation rules (`no_std`); repairability **scoring** belongs here, not there |
| `dpp-plugin-sdk` | Sector plugins import `dpp-calc` for calculator functions (Phase 2) |
| `dpp-engine` (BSL-1.1) | Stores `CalculationReceipt`, serves the verification endpoint, manages `FactorProvider` lifecycle |

---

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
