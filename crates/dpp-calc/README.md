# dpp-calc

[![crates.io](https://img.shields.io/crates/v/dpp-calc.svg)](https://crates.io/crates/dpp-calc)
[![docs.rs](https://img.shields.io/docsrs/dpp-calc)](https://docs.rs/dpp-calc)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

Pure, stateless EU-methodology compliance calculators for [Odal Node](https://odal-node.io).

Every function in this crate is deterministic and side-effect free — no I/O, no infrastructure, no `async`.
The math is open-source (Apache-2.0). Licensed lifecycle-inventory (LCI) data is injected at runtime
through the `FactorProvider` trait and never bundled here — open methodology, licensed data supplied
by the operator. That split is the licensing rationale for this crate.

---

## When to use this crate

- You are a **host** running a determination for a product group and need it to carry a ruleset id,
  an effective period, a legal citation and a receipt.
- You are writing an integration test and need golden-vector inputs and expected outputs.
- You are adding a new EU methodology or product category (see [Adding a product group calculator](#adding-a-product group-calculator) below).

## When NOT to use this crate

- You are a **Wasm product group plugin**. This crate is not reachable from one, deliberately:
  the plugins are a separate workspace targeting `wasm32-wasip1`, and pulling `chrono`,
  `uuid`, `sha2` and `serde_jcs` into every plugin binary to mint a receipt inside a
  sandbox — one that would have to be recompiled and re-signed for every threshold
  change — is the arrangement the signed ruleset channel exists to avoid. Plugins reach
  thresholds through `dpp-rules`, which is `no_std` and re-exported by `dpp-plugin-sdk`.
- You need the DPP data model or port traits → `dpp-domain`.
- You need the Wasm plugin ABI → `dpp-plugin-traits` / `dpp-plugin-sdk`.
- You need field-level validation rules (REACH, fibre percentages, voltage ranges) → `dpp-rules`.

---

## Module structure

```text
src/
├── lib.rs                    public API + product group-calculator scaling guide (read this first)
│
├── kernel/                   machinery shared by every methodology
│   ├── error.rs              CalcError (InvalidInput | RulesetExpired | FactorNotFound | …)
│   ├── receipt.rs            CalculationReceipt — proof-of-calculation envelope
│   ├── ruleset.rs            RulesetId, RulesetVersion, Effectivity, RegulatoryBasis
│   │                         ParameterBasis — are these numbers law, or ours?
│   │                         Ruleset trait — every methodology trait extends this
│   ├── parameters.rs         RulesetParameters + fill(): a signed bundle may fill
│   │                         an Assumed ruleset's numbers, never a Sourced one's
│   ├── clock.rs              AssessmentClock — the governing-law date; there is no now()
│   ├── assessability.rs      Assessability<T>: Assessed | NotYetInForce | Undetermined
│   │                         | Expired | OutOfScope — none of which is non-compliance
│   ├── factor.rs             FactorProvider trait
│   ├── synthetic_factor.rs   SyntheticFactorProvider (test/CI only)
│   └── hashing.rs            canonical input hashing behind input_hash
│
├── ruleset_registry/         date-based ruleset resolution; all_rulesets() CI iterator
│
├── recycled_content/         EU 2023/1542 Art. 8 minimum recycled shares
│   ├── calculator.rs         calculate(inputs, ruleset, clock) → RecycledContentResult
│   ├── parameters.rs         RecycledContentInputs — the four declared shares
│   ├── thresholds.rs         RecycledContentRuleset + Art8Phase1Ruleset / Art8Phase2Ruleset
│   └── golden_vectors.rs     #[cfg(test)], incl. a guard that this agrees with dpp-rules
│
├── repairability_index/      The enacted EU 2023/1669 Annex IV index
│   ├── calculator.rs         calculate(inputs, ruleset) → RepairabilityIndexResult
│   ├── parameters.rs         RepairabilityIndexInputs — 3 part-level parameters over the
│   │                         10 Annex IV priority parts + 3 product-level, each scored 1–5
│   ├── thresholds.rs         RepairabilityIndexRuleset + Eu2023_1669Ruleset
│   └── golden_vectors.rs     #[cfg(test)]
│
├── repairability/            Simplified, non-regulatory six-parameter A–E heuristic
│   ├── calculator.rs         calculate(inputs, ruleset, clock) → RepairabilityResult
│   ├── parameters.rs         RepairabilityInputs (6 × u8, ordinal 0–2)
│   ├── thresholds/           RepairabilityRuleset trait + SimplifiedRepairabilityHeuristic,
│   │                         LaptopRuleset, DisplaysRuleset, WashingMachineRuleset
│   └── golden_vectors.rs     #[cfg(test)] — A–E coverage, regulatory-basis non-empty CI check
│
└── co2e/
    ├── calculator.rs         calculate(inputs, ruleset, clock) → Co2eResult (cradle-to-gate,
    │                         operator-supplied emission factors)
    ├── parameters.rs         Co2eInputs, MaterialFootprint
    ├── thresholds.rs         Co2eRuleset trait + CradleToGateRuleset
    ├── cfb.rs                CfbRuleset: Ruleset stub + calculate_cfb() STUB (Phase 2)
    ├── gwp_factors.rs        Embedded GWP100 characterisation factors (EF 3.1 / AR6 — free)
    └── golden_vectors.rs     #[cfg(test)]
```

**Phase status:**

| Module | Status |
|---|---|
| `recycled_content` | ✅ Enacted — Reg. (EU) 2023/1542 Art. 8(2) (from 2031-08-18) and Art. 8(3) (from 2036-08-18); both resolve as `NotYetInForce` for a battery placed on the market today, which is the correct answer, not a gap |
| `repairability_index` | ✅ Enacted — Reg. (EU) 2023/1669 Annex IV point 5, smartphones and slate tablets |
| `repairability` | ✅ Available — non-regulatory six-parameter heuristic; **not** the enacted index |
| `co2e::calculate` | ✅ Baseline — operator-supplied emission factors |
| `co2e::cfb` | 🔒 Stub — gated on signed ecoinvent/EF sublicense (Phase 1 gate) |
| `pef/` (future) | 📋 Not yet — awaits per-product group PEFCR finalisation (2026–2030) |

### Which repairability module do I want?

`repairability_index` implements the scoring a delegated act actually prescribes,
and is what belongs in a passport field that claims to be *the* repairability
index. `repairability` is an internal six-factor heuristic that predates it and
covers product categories the regulation does not reach (laptops, displays,
washing machines). They are separate modules, not versions of each other: the
inputs, the scale and the legal standing all differ. Note that only the
heuristic takes an `AssessmentClock` — the index has a single ruleset, so there
is no date-dependent choice to make.

---

## Example

### Repairability index (enacted — EU 2023/1669 Annex IV)

```rust
use chrono::NaiveDate;
use dpp_calc::repairability_index::{
    Eu2023_1669Ruleset, PriorityPartScores, RepairabilityIndexInputs, calculate,
};
use dpp_calc::ruleset_registry;

fn assess() -> Result<(), Box<dyn std::error::Error>> {
// Each of the three part-level parameters is scored 1–5 for all ten Annex IV
// priority parts. `folding_mechanism` is None for a non-foldable product.
let parts = |s: u8| PriorityPartScores {
    battery: s,
    display_assembly: s,
    back_cover: s,
    front_camera: s,
    rear_camera: s,
    charging_port: s,
    mechanical_button: s,
    microphone: s,
    speaker: s,
    folding_mechanism: None,
};

let inputs = RepairabilityIndexInputs {
    disassembly_depth:  parts(4),
    fasteners:          parts(5),
    tools:              parts(4),
    spare_parts:        4,
    software_updates:   5,
    repair_information: 3,
};

let result = calculate(&inputs, &Eu2023_1669Ruleset)?;
println!("R = {:.2} → class {:?}", result.index, result.class);

// Or resolve the governing ruleset by product category and law date:
let law_date = NaiveDate::from_ymd_opt(2026, 7, 25).unwrap();
if let Some(ruleset) =
    ruleset_registry::resolve_repairability_index("smartphone-tablet", law_date).assessed()
{
    let result = calculate(&inputs, ruleset)?;
    println!("R = {:.2}", result.index);
}
Ok(())
}
```

`resolve_repairability_index` returns an `Assessability`, not an `Option`:
`NotYetInForce`, `Undetermined`, `Expired` and `OutOfScope` are distinct answers,
and **none of them means non-compliant**.

### Repairability (simplified non-regulatory heuristic, A–E)

```rust
use chrono::NaiveDate;
use dpp_calc::clock::AssessmentClock;
use dpp_calc::repairability::{
    RepairabilityInputs, SimplifiedRepairabilityHeuristic, calculate,
};
use dpp_calc::ruleset_registry;

fn assess() -> Result<(), Box<dyn std::error::Error>> {
// The date the governing law attached to this product — read from the product's
// own record (`placedOnMarketDate`), never from the wall clock. There is
// deliberately no `AssessmentClock::now()`.
let placed_on_market = NaiveDate::from_ymd_opt(2026, 3, 14).unwrap();
let clock = AssessmentClock::placed_on(placed_on_market);

let inputs = RepairabilityInputs {
    disassembly:           2,
    spare_parts:           2,
    repair_info:           1,
    diagnostic_tools:      1,
    software_updatability: 2,
    customer_support:      1,
};

// Option A: use the ruleset directly (when you know the product category at compile time)
let result = calculate(&inputs, &SimplifiedRepairabilityHeuristic, clock)?;
println!("{} ({:.2}/10)", result.class, result.numeric_score); // "B (7.75/10)"
println!("receipt: {}", result.receipt.receipt_id);

// Option B: resolution by governing-law date (when the category comes from a passport field)
let ruleset = ruleset_registry::resolve_repairability("smartphone-tablet", clock.law_in_force_on)
    .assessed()
    .ok_or("no ruleset governs this category on that date")?;
let result = calculate(&inputs, ruleset, clock)?;
Ok(())
}
```

### Cradle-to-gate CO₂e

```rust
use chrono::NaiveDate;
use dpp_calc::clock::AssessmentClock;
use dpp_calc::co2e::{Co2eInputs, CradleToGateRuleset, MaterialFootprint, calculate};

fn assess() -> Result<(), Box<dyn std::error::Error>> {
let clock = AssessmentClock::placed_on(NaiveDate::from_ymd_opt(2026, 3, 14).unwrap());

let result = calculate(
    &Co2eInputs {
        materials: vec![
            MaterialFootprint { mass_kg: 0.5, emission_factor_kg_co2e_per_kg: 8.0 },
            MaterialFootprint { mass_kg: 0.2, emission_factor_kg_co2e_per_kg: 3.0 },
        ],
        energy_kwh: 1.5,
        grid_factor_kg_co2e_per_kwh: 0.4,
    },
    &CradleToGateRuleset,
    clock,
)?;
println!("{:.2} kg CO₂e", result.total_co2e_kg); // 5.20 kg CO₂e
Ok(())
}
```

---

## CalculationReceipt

Every calculator returns a `CalculationReceipt` alongside the computed value.
Shape shown for orientation; the definition is in `kernel::receipt`:

```rust,ignore
pub struct CalculationReceipt {
    pub receipt_id:             Uuid,          // UUIDv7 — time-sortable
    pub input_hash:             String,        // SHA-256 of canonical JSON inputs
    pub ruleset_id:             String,        // e.g. "repairability-heuristic-v1"
    pub ruleset_version:        String,        // e.g. "1.0.0"
    /// SHA-256 of the parameter set actually used — which *numbers*, where
    /// `ruleset_id` + `ruleset_version` say which *rule*.
    pub ruleset_content_sha256: String,
    /// The signed bundle that filled those parameters, if any. Both `None` for
    /// the compiled-in baseline, and they only ever move together.
    pub bundle_version:         Option<String>,
    pub bundle_content_sha256:  Option<String>,
    pub factor_dataset_id:      String,        // empty if no FactorProvider used
    pub factor_dataset_version: String,
    pub factor_set_hash:        Option<String>, // SHA-256 of full factor table
    /// The date whose law the calculation was performed against.
    pub assessed_as_of: NaiveDate,
    pub computed_at:            DateTime<Utc>,
}
```

The platform stores this alongside the passport record. A notified body can re-run
any calculation from the receipt: same inputs (verify via `input_hash`) + same
parameters (verify via `ruleset_content_sha256`) + same factor dataset version →
must produce the same output.

The parameter hash is what makes that a check rather than a claim. A version
string is maintained by hand, so two receipts citing the same `ruleset_version`
from builds carrying different thresholds used to be indistinguishable.

---

## RegulatoryBasis

Every concrete ruleset carries a machine-readable legal citation:

```rust
use dpp_calc::ruleset::Ruleset;
use dpp_calc::repairability_index::Eu2023_1669Ruleset;

let _basis = Eu2023_1669Ruleset.regulatory_basis();
// RegulatoryBasis {
//   regulation: "EU 2023/1669",
//   article:    "Annex IV point 5 (calculation method); \
//                Annex II Table 4 (class boundaries)",
//   standard:   Some("EN 45554:2020"),
//   …
// }
```

A non-regulatory ruleset says so in the same field rather than leaving it blank —
`SimplifiedRepairabilityHeuristic` reports its `regulation` as
`"Non-regulatory: simplified repairability heuristic (NOT EU 2023/1669 Annex IV)"`,
so a receipt can never imply legal standing the calculation does not have.

A CI test (`expired_rulesets_have_superseded_by`) asserts that any ruleset with
`Effectivity::InForce.until < today` has a non-empty `superseded_by`. This keeps the audit chain intact as regulations evolve.

---

## Feature flags

| Flag | Default | Purpose |
|---|---|---|
| `synthetic-factors` | off | Exposes `SyntheticFactorProvider` outside `#[cfg(test)]`. Enable in integration test harnesses only. Values are **not** real LCI data. |
| `real-factors` | off | Gates licensed LCI factor-data implementations. Enable **only** in managed-service builds where a valid ecoinvent/EF reseller sublicense is in place. Never enable in open-source or self-hosted builds. |

---

## Adding a product group calculator

See the `# Adding a new product group calculator` section in `src/lib.rs` for the full
step-by-step guide. Short version:

1. **New methodology** → add `src/{methodology}/` with `mod.rs`, `calculator.rs`,
   `parameters.rs`, `thresholds.rs` (trait extends `Ruleset`), `golden_vectors.rs`.
   Register in `src/ruleset_registry/resolve.rs`.
2. **New product category on an existing methodology** → add `impl Ruleset + impl {Methodology}Ruleset`
   in that methodology's `thresholds` module (a file, or a directory with one
   file per category as in `repairability/thresholds/`), add a row to the
   registry, add golden vectors.
3. **Pending delegated act** → use `Effectivity::pending(empowerment, adoption_deadline)`. There is no date sentinel: a pending ruleset has no application date and resolves for no date at all.
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
| `dpp-rules` | Field-level validation rules and regulatory thresholds (`no_std`, zero-dep). **`dpp-calc` depends on it**, never the reverse: a threshold like the Art. 8 minimum shares has one home, and it is there, because the Wasm product group plugins reach it too and cannot reach this crate. What `dpp-calc` adds on top is the ruleset identity, the effective period and the receipt — none of which a `no_std` crate can produce |
| `dpp-plugin-sdk` | Does **not** re-export this crate, and is not planned to — see "When NOT to use this crate". Plugins reach regulatory thresholds through `dpp-rules` |
| `dpp-engine` (BSL-1.1) | Stores `CalculationReceipt`, serves the verification endpoint, manages `FactorProvider` lifecycle |

---

## Minimum Rust version

1.96 (MSRV is enforced in CI)

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
