# Regulatory Implementation Status

This document is the single reference for why a rule module contains real logic vs. a placeholder. Each entry states the legal basis, what is and isn't finalized, and the concrete condition that unlocks implementation.

---

## How to read this document

**Implemented** — rule functions exist in `dpp-rules`, are tested, and are called by the relevant plugin or domain adapter.

**Pending** — the regulation is in force but the specific threshold/methodology is in a delegated act that has not yet been adopted. The module exists with constants or stubs and a placeholder comment citing this document.

**Not mandated** — there is no EU DPP requirement yet for this sector. Placeholder module exists so the directory structure is explicit about what will be needed.

---

## Batteries — EU Regulation 2023/1542

### `batteries/chemistry.rs` — Implemented ✅

| Rule | Threshold | Legal basis | Status |
|---|---|---|---|
| Mercury prohibition | > 0.0005 % by weight → prohibited | Art. 9 / Batteries Directive 2006/66/EC Art. 4(1) | In force since 2008 |
| Cadmium prohibition (portable) | > 0.002 % by weight → prohibited | Art. 9 / Batteries Directive 2006/66/EC Art. 4(2) | In force since 2008 |
| Operating temp range | `minC` must be < `maxC` | Annex XIII (cross-field coherence) | Always applicable |

Mercury and cadmium thresholds are inherited from the old Batteries Directive and explicitly carried forward into EU 2023/1542. They are **not** pending any delegated act. Exceptions for emergency/alarm systems and medical devices are being phased out under Art. 88.

The operating temperature cross-field check has no regulatory threshold — it is a data-coherence rule that JSON Schema cannot express across two fields.

**When to update:** No update needed for these thresholds unless Art. 9 is amended by a subsequent regulation.

---

### `batteries/recycled_content.rs` — Constants implemented, determination pending ⏳

The Art. 8(2)/(3) recycled content targets are **finalized law** — they appear in the regulation text itself, not in a delegated act. However, neither phase is yet in force. Art. 8(2) and 8(3) both point to **Annex VIII** (technical documentation) as the place the share must be demonstrated.

| Metal | Phase 1 (from 18 Aug 2031) | Phase 2 (from 18 Aug 2036) | Measured in |
|---|---|---|---|
| Cobalt | 16 % | 26 % | active materials |
| Lead | 85 % | 85 % | the battery |
| Lithium | 6 % | 12 % | active materials |
| Nickel | 6 % | 15 % | active materials |

**Scope.** Phase 1 covers industrial batteries > 2 kWh (excluding those with exclusively external storage), EV batteries and **SLI batteries**. Phase 2 adds **LMT batteries**. Portable batteries are out of scope throughout. SLI is *in* Phase-1 scope — an earlier version of this table wrongly excluded it and wrongly placed LMT in Phase 1.

**Dates.** 18 August, not 1 January — an earlier version of this table had both phase dates wrong by seven and a half months.

The module exposes `art8_shortfalls_2031` and `art8_shortfalls_2036` (renamed from `annex_x_shortfalls_*` on 2026-07-25, when the citation was corrected). The battery plugin returns `NOT_ASSESSED` because neither phase is in force yet. When 2031 arrives, the plugin switches from `NOT_ASSESSED` to a real determination by calling these functions — **no change to `dpp-rules` is needed at that point**.

**Art. 8(1) — the declaration duty, now modelled.** Separate from the minimums above: it requires documentation of the *actual* shares, with no minimum, from 18 Aug 2028 (industrial > 2 kWh / EV / SLI) and 18 Aug 2033 (LMT) — "or 24 months after the date of entry into force of the delegated act …, whichever is the latest". Annex XIII point 1(e) makes that documentation publicly accessible passport content.

Because that date is conditional and the act is unadopted, `art8_declaration_duty_for` returns one of three answers rather than a yes/no: `NotYetDue` below the floor (certain, since the real date can only be the floor or later), `Undetermined` on or after it, and `NotCovered` outside scope.

Art. 8(1) also requires the shares "for each battery model **per year and per manufacturing plant**". A percentage without both anchors is not the Art. 8(1) declaration, so schema v2.3.0 adds `recycledContentReportingYear` to pair with the existing `manufacturingPlace`, and the battery plugin flags shares declared without it.

**When to update:** Only if Art. 8 is amended by a subsequent regulation, or when the Art. 8(1) methodology delegated act (due 18 Aug 2026) is adopted.

---

### `batteries/degradation.rs` — Pending delegated act ⏳

Two regimes on two timelines — only the second is pending.

**Already in force.** Art. 10(1) (since 18 Aug 2024) requires industrial batteries > 2 kWh, LMT and EV batteries to be accompanied by a document carrying the Annex IV Part A parameters. Art. 14(1) (since 18 Aug 2024) requires the Annex VII parameters to be held in the battery management system. Both are declaration duties, not thresholds.

**Pending — minimum values, Art. 10(5).**

| Scope | Delegated act due | Minimum values apply from |
|---|---|---|
| Industrial > 2 kWh (excl. exclusively external storage) | 18 Feb 2026 | 18 Aug 2027 |
| LMT batteries | 18 Feb 2027 | 18 Aug 2028 |

Both application dates are conditional — Art. 10(2)/(3) read "or 18 months after the date of entry into force of the delegated act, whichever is the latest".

**Art. 10(6) is not the empowerment.** It lets the Commission amend the Annex IV *parameter list*. An earlier version of this document cited it for the minimum values; corrected 2026-07-25.

**Art. 10(4)** disapplies paragraphs 1–3 for batteries prepared for re-use, repurposed or remanufactured that were placed on the market before the obligations applied — so a determination keys on the original placing-on-market date.

**Annex VII Part A is now modelled** (schema v2.2.0, `BatteryData::state_of_health`). It is two disjoint lists, so the type is a sum, not a struct of optionals:

| Category | Parameters |
|---|---|
| Electric vehicle | state of certified energy (SOCE) — and nothing else |
| Stationary storage + LMT | remaining capacity and evolution of self-discharging rates (unconditional); remaining power capability, remaining round trip efficiency, ohmic resistance (each "where possible") |

`dpp_rules::batteries::degradation::annex_vii_parameter_set_for` maps a battery type onto the applicable list. Portable and SLI return `None` — Art. 14(1) names only stationary storage, LMT and EV batteries.

Annex XIII point 4(b) puts state of health in the individual-battery set, so `stateOfHealth` and the deprecated `stateOfHealthPct` both carry the `individual` disclosure class: legitimate-interest holders see them, authorities do not. Before v2.2.0 neither field was classified, so both defaulted to public.

The flat `stateOfHealthPct` is retained for schema versions up to v2.1.0 and cannot represent either list.

**When to update:** When the Art. 10(5) delegated acts are published. At that point, implement the minimum values here keyed by battery category, and switch the battery plugin from `NOT_ASSESSED` to a real determination for the affected categories.

---

### Battery CO₂e carbon footprint class — Pending delegated act ⏳

The battery schema carries `carbonFootprintClass`. Art. 7(2) defines **no class labels** — it defers both the classes and the methodology to a **Commission Delegated Regulation under Art. 7(2)** that has not been adopted, and requires the Commission to "review the number of performance classes and the thresholds between them, every three years". The schema's `A–E` enumeration was therefore never sourced from the regulation; it is pending removal. Treat the class label as an opaque string carried with the ruleset id and version that produced it.

**Not in `dpp-rules` scope.** The class assignment is a calculation, not a cross-field validation rule. When the delegated act is published, implement the class-boundary thresholds in `dpp-calc` (the calculator crate), not here.

---

## Textiles — EU ESPR Textile DPP Delegated Act

### `textiles/fibre.rs` — Implemented ✅

| Rule | Logic | Status |
|---|---|---|
| Fibre sum | Percentages must sum to ~100 % (± 2 pp tolerance) | Implemented |
| Per-fibre range | Each `pct` in [0, 100] | Implemented |
| Country of origin | Must be valid ISO 3166-1 alpha-2 | Implemented |

These are structural data-coherence rules derived from the schema requirements. They are not dependent on a specific delegated act.

---

### `textiles/care.rs` — Schema change needed 🔧

The textile schema v1.1.0 carries `careInstructions` as a **free-text string**. There is no structured array of individual care symbol objects. Cross-field validation of ISO 3758:2012 symbol codes is not applicable until the schema introduces a structured field.

ISO 3758:2012 is a published standard — the symbols themselves are well-defined. The blocker is schema design, not regulation.

**When to update:** When the textile schema adds a structured `careSymbols` array (e.g. `[{category: "washing", tempC: 40}, ...]`). At that point, implement:
- `washing_temperature_valid(temp_c)` — allowed values: 30, 40, 60, 70, 95
- `care_treatment_valid(treatment)` — validates against the ISO 3758 symbol set
- Cross-field: if a washing temperature is declared, a washing method must also be present

---

## Electronics — EU Electronics DPP (adopted 18 March 2026)

### `electronics/spare_parts.rs` — Schema change needed 🔧

The electronics schema v1.0.0 carries `sparePartsAvailable: bool` — a binary yes/no field. There is no availability *period* field. Period-based rules cannot be implemented against a boolean.

Minimum availability periods from pre-ESPR ecodesign implementing regulations (for reference):

| Product category | Minimum period | Source |
|---|---|---|
| Washing machines / dryers | 10 years | EU 2019/2022 |
| Dishwashers | 10 years | EU 2019/2022 |
| Refrigerators / freezers | 7–10 years | EU 2019/2019 |
| Displays / TVs | 7–10 years | EU 2019/2021 |
| Smartphones / laptops | pending | ESPR delegated act TBD |
| Servers | pending | ESPR delegated act TBD |
| All other categories | pending | ESPR delegated act TBD |

**When to update:** Two conditions must both be met:
1. The electronics schema gains a `sparePartsAvailabilityYears` (or equivalent) field.
2. The ESPR delegated act specifies minimum periods per product category.

At that point, implement `validate_spare_parts_period(years: u32, category: &str) -> Result<(), String>` here, keyed by the `productCategory` enum values from the schema.

---

## Metals

### `metals/aluminium.rs` — CBAM benchmarks implemented, DPP mandate not finalized ⏳

| Rule | Threshold (kg CO₂e / t) | Status |
|---|---|---|
| Primary route | ≤ 10 000 | CBAM benchmark — **not** a DPP mandate |
| Secondary-recycled route | ≤ 1 000 | CBAM benchmark — **not** a DPP mandate |
| Mixed route | ≤ 5 000 | CBAM benchmark — **not** a DPP mandate |

CBAM (EU 2023/956) covers embedded-carbon reporting for aluminium imports but does not set production-level CO₂e thresholds that create a DPP compliance obligation. The thresholds above are industry/CBAM reference values used by the `sector-aluminium` plugin.

The aluminium DPP mandate is expected around 2030.

`co2e_within_route_threshold` exists today so the plugin has a single source of truth and does not hardcode the values. The plugin returns `NOT_ASSESSED`. When the DPP mandate is finalized, update the constants here and update the plugin to return a real determination — the function call site stays the same.

**When to update:** When an ESPR delegated act specifies mandatory CO₂e thresholds for the aluminium sector DPP.

---

### `metals/steel.rs` — Reference intensities only, no DPP mandate ⏳

No steel DPP mandate is currently in force. CBAM (EU 2023/956) covers embedded-carbon reporting for steel imports but sets no per-tonne CO₂e threshold that creates a compliance obligation.

Reference CO₂e intensities per route (worldsteel / IEA, not mandated thresholds):

| Route | Typical range (tCO₂e / t steel) |
|---|---|
| `blast-furnace` (BF-BOF) | 1.8 – 2.5 |
| `electric-arc` (EAF, scrap-based) | 0.3 – 0.7 |
| `direct-reduction` (DRI-EAF) | 0.1 – 1.4 |

The `sector-steel` plugin exists and validates structure + records metrics, but `dpp-rules` carries no steel compliance-checking function yet (only the reference constants above). The plugin returns `NOT_ASSESSED` for the determination, mirroring aluminium.

**When to update:** When an ESPR delegated act specifies mandatory CO₂e thresholds for the steel sector DPP. At that point, implement `co2e_within_route_threshold` here (modelled on the aluminium equivalent) and switch the `sector-steel` plugin from `NOT_ASSESSED` to a real determination — the plugin already exists, so only the rule function and the plugin's determination call change.

---

## Placeholder sectors

### Construction — `construction/mod.rs`

| Regulation | Status | DPP mandate |
|---|---|---|
| EU CPR 2024/3110 (Construction Products Regulation) | In force | 2028–2032 (phased by product family) |

No cross-field rules are implementable until the delegated acts under CPR 2024/3110 specify DPP field requirements and compliance thresholds per construction product family (cement, aggregates, structural steel elements, etc.).

**When to update:** When the first CPR 2024/3110 delegated act specifying DPP data requirements is published. Add sub-modules per product family (e.g. `construction/concrete.rs`, `construction/structural.rs`) following the same pattern as the battery sub-modules.

---

### Toys — `toys/mod.rs`

| Regulation | Status | DPP mandate |
|---|---|---|
| EU Toy Safety Regulation 2025/2509 | In force | ~2030 |

The CE marking check (one hard rule that is available today) lives in the `sector-toy` plugin, not in `dpp-rules`, because it is a single-field boolean check that JSON Schema can enforce directly. There are no cross-field regulatory rules to implement in `dpp-rules` until the DPP delegated act is published.

**When to update:** When the EU 2025/2509 delegated act specifying toy DPP data requirements is published.

---

## Summary table

| Module | State | Unblocked by |
|---|---|---|
| `batteries/chemistry.rs` | ✅ Implemented | — |
| `batteries/recycled_content.rs` | ✅ Constants + functions ready | Determination switches at 2031 (no code change needed) |
| `batteries/degradation.rs` | ⏳ Pending | Art. 10(5) delegated acts (due 18 Feb 2026 / 18 Feb 2027) |
| `batteries` CO₂e class | ⏳ Pending (→ dpp-calc) | Art. 7(2) delegated act |
| `textiles/fibre.rs` | ✅ Implemented | — |
| `textiles/care.rs` | 🔧 Schema change needed | Structured `careSymbols` field in textile schema |
| `electronics/spare_parts.rs` | 🔧 Schema change needed | Period field in electronics schema + ESPR delegated act |
| `metals/aluminium.rs` | ✅ Benchmarks only | Full DPP threshold: ESPR delegated act (~2030) |
| `metals/steel.rs` | ⏳ Pending | ESPR delegated act (timeline unknown) |
| `construction/mod.rs` | ⏳ Not mandated yet | CPR 2024/3110 delegated acts (2028–2032) |
| `toys/mod.rs` | ⏳ Not mandated yet | EU 2025/2509 delegated act (~2030) |
