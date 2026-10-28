# Regulatory implementation status

<!-- BOUNDARY-EXCEPTION(taxonomy): "Phase 1" and "Phase 2" are the stages of Regulation (EU) 2023/1542 Art. 8(2) and 8(3), not a roadmap. -->

The single reference for why a rule module holds real logic or a placeholder. Each entry
gives the legal basis, what is and is not settled, and the condition that unblocks
implementation.

---

## Status labels

**Implemented:** rule functions exist in `dpp-rules`, are tested, and are called by the
relevant plugin or domain adapter.

**Pending:** the regulation is in force, but the specific threshold or methodology sits in a
delegated act that has not been adopted. The module holds constants or stubs and a comment
citing this document.

**Schema change needed:** the rule is known, but the current schema cannot carry the data it
would check.

**Not mandated:** there is no EU passport requirement for this product group yet. A
placeholder module makes the directory structure explicit about what will be needed.

---

## Batteries: Regulation (EU) 2023/1542

### `batteries/chemistry.rs`: implemented

| Rule | Threshold | Legal basis | Status |
|---|---|---|---|
| Mercury prohibition | more than 0.0005 % by weight is prohibited | Art. 9; Batteries Directive 2006/66/EC Art. 4(1) | In force since 2008 |
| Cadmium prohibition (portable) | more than 0.002 % by weight is prohibited | Art. 9; Batteries Directive 2006/66/EC Art. 4(2) | In force since 2008 |
| Operating temperature range | `minC` must be below `maxC` | Annex XIII (cross-field coherence) | Always applicable |

The mercury and cadmium thresholds come from the old Batteries Directive and are carried
forward explicitly into Regulation (EU) 2023/1542. They do **not** depend on any delegated
act. The exceptions for emergency and alarm systems and medical devices are being phased
out under Art. 88.

The operating temperature check has no regulatory threshold: it is a data-coherence rule
that JSON Schema cannot express across two fields.

**When to update:** only if Art. 9 is amended by a later regulation.

---

### `batteries/recycled_content.rs`: constants implemented, determination pending

The Art. 8(2) and 8(3) recycled content targets are **settled law**: they are in the
regulation text itself, not in a delegated act. Neither phase is in force yet. Art. 8(2) and
8(3) both point to **Annex VIII** (technical documentation) as where the share must be
demonstrated.

| Metal | Phase 1 (from 18 Aug 2031) | Phase 2 (from 18 Aug 2036) | Measured in |
|---|---|---|---|
| Cobalt | 16 % | 26 % | active materials |
| Lead | 85 % | 85 % | the battery |
| Lithium | 6 % | 12 % | active materials |
| Nickel | 6 % | 15 % | active materials |

**Scope.** Phase 1 covers industrial batteries over 2 kWh (except those with exclusively
external storage), EV batteries and **SLI batteries**. Phase 2 adds **LMT batteries**.
Portable batteries are out of scope throughout. SLI is *in* Phase 1 scope; an earlier
version of this table wrongly excluded it and wrongly placed LMT in Phase 1.

**Dates.** 18 August, not 1 January. An earlier version of this table had both phase dates
wrong by seven and a half months.

The module exposes `art8_shortfalls_2031` and `art8_shortfalls_2036` (renamed from
`annex_x_shortfalls_*` on 2026-07-25, when the citation was corrected). The battery plugin
returns `NOT_ASSESSED` because neither phase is in force. When 2031 arrives, the plugin
switches from `NOT_ASSESSED` to a real determination by calling these functions; **no change
to `dpp-rules` is needed then**.

**Art. 8(1), the declaration duty, is modelled.** Separate from the minimums above, it
requires documentation of the *actual* shares, with no minimum, from 18 August 2028
(industrial over 2 kWh, EV, SLI) and 18 August 2033 (LMT), "or 24 months after the date of
entry into force of the delegated act …, whichever is the latest". Annex XIII point 1(e)
makes that documentation publicly accessible passport content.

Because the date is conditional and the act is unadopted, `art8_declaration_duty_for`
returns one of three answers rather than a yes or no: `NotYetDue` before the floor (certain,
since the real date can only be the floor or later), `Undetermined` on or after it, and
`NotCovered` outside scope.

Art. 8(1) also requires the shares "for each battery model **per year and per manufacturing
plant**". A percentage without both anchors is not the Art. 8(1) declaration, so schema
v2.3.0 adds `recycledContentReportingYear` to pair with the existing `manufacturingPlace`,
and the battery plugin flags shares declared without it.

**When to update:** only if Art. 8 is amended by a later regulation, or when the Art. 8(1)
methodology delegated act (due 18 August 2026) is adopted.

---

### `batteries/degradation.rs`: pending delegated act

Two regimes on two timelines; only the second is pending.

**Already in force.** Art. 10(1) (since 18 August 2024) requires industrial batteries over
2 kWh, LMT and EV batteries to come with a document carrying the Annex IV Part A parameters.
Art. 14(1) (since 18 August 2024) requires the Annex VII parameters to be held in the battery
management system. Both are declaration duties, not thresholds.

**Pending: minimum values, Art. 10(5).**

| Scope | Delegated act due | Minimum values apply from |
|---|---|---|
| Industrial over 2 kWh (except exclusively external storage) | 18 Feb 2026 | 18 Aug 2027 |
| LMT batteries | 18 Feb 2027 | 18 Aug 2028 |

Both application dates are conditional: Art. 10(2) and (3) read "or 18 months after the date
of entry into force of the delegated act, whichever is the latest".

**Art. 10(6) is not the empowerment.** It lets the Commission amend the Annex IV *parameter
list*. An earlier version of this document cited it for the minimum values; corrected
2026-07-25.

**Art. 10(4)** disapplies paragraphs 1 to 3 for batteries prepared for re-use, repurposed or
remanufactured that were placed on the market before the obligations applied, so a
determination depends on the original placing-on-market date.

**Annex VII Part A is modelled** (schema v2.2.0, `BatteryData::state_of_health`). It is two
disjoint lists, so the type is a sum, not a struct of optionals:

| Category | Parameters |
|---|---|
| Electric vehicle | state of certified energy (SOCE), and nothing else |
| Stationary storage and LMT | remaining capacity and evolution of self-discharging rates (unconditional); remaining power capability, remaining round trip efficiency, ohmic resistance (each "where possible") |

**Annex VII Part B (expected lifetime) is modelled too** (schema v2.4.0,
`BatteryData::expected_lifetime`): date of putting into service ("where appropriate"),
energy throughput, capacity throughput, tracking of harmful events, and the number of full
equivalent charge-discharge cycles.

**Part B is narrower than Part A.** Part A names *"electric vehicle batteries, stationary
battery energy storage systems and LMT batteries"*; Part B names only the latter two. An EV
battery reports a state of health but **no** expected-lifetime parameter set; see
`annex_vii_part_b_applies_to`.

Part B item 4 lists its harmful events *"such as"*, so that set is illustrative and every
member is optional. An implementation that tracks a further event type is conforming, not
extending.

`expectedLifetime` (measured, per item) is `individual` under Annex XIII point 4(d). It is
deliberately distinct from `expectedLifetimeCycles`, the model-level design figure Annex
XIII point 1(j) makes **public**.

`dpp_rules::batteries::degradation::annex_vii_parameter_set_for` maps a battery type to the
applicable Part A list. Portable and SLI return `None`: Art. 14(1) names only stationary
storage, LMT and EV batteries.

Annex XIII point 4(b) puts state of health in the individual-battery set, so `stateOfHealth`
and the deprecated `stateOfHealthPct` both carry the `individual` disclosure class:
legitimate-interest holders see them, authorities do not. Before v2.2.0 neither field was
classified, so both defaulted to public.

The flat `stateOfHealthPct` is kept for schema versions up to v2.1.0 and cannot represent
either list.

**When to update:** when the Art. 10(5) delegated acts are published. Then implement the
minimum values here, keyed by battery category, and switch the battery plugin from
`NOT_ASSESSED` to a real determination for the affected categories.

---

### Battery carbon footprint class: pending delegated act

The battery schema carries `carbonFootprintClass`. Art. 7(2) defines **no class labels**: it
defers the classes to a **Commission Delegated Regulation under Art. 7(2)** that has not
been adopted. The calculation methodology the classes are read against is a separate
delegated act, under Art. 7(1), fourth subparagraph, point (a). Art. 7(2) also requires the
Commission to "review the number of
performance classes and the thresholds between them, every three years". The schema's `A–E`
enumeration was therefore never sourced from the regulation, and is due for removal. Treat
the class label as an opaque string carried with the ruleset id and version that produced
it.

**Not in `dpp-rules` scope.** Assigning a class is a calculation, not a cross-field
validation rule. When the delegated act is published, implement the class boundaries in
`dpp-calc`, not here.

---

## Textiles: the ESPR textile delegated act

### `textiles/fibre.rs`: implemented

| Rule | Logic | Status |
|---|---|---|
| Fibre sum | Percentages sum to about 100 % (± 2 percentage points) | Implemented |
| Per-fibre range | Each `pct` in [0, 100] | Implemented |
| Country of origin | A valid ISO 3166-1 alpha-2 code | Implemented |

These are structural data-coherence rules derived from the schema. They do not depend on a
specific delegated act.

---

### `textiles/care.rs`: schema change needed

The textile schema carries `careInstructions` as a **free-text string**. There is no
structured list of care symbols, so cross-field validation of ISO 3758:2012 symbol codes
cannot apply until the schema adds a structured field.

ISO 3758:2012 is published and its symbols are well defined. The blocker is schema design,
not regulation.

**When to update:** when the textile schema adds a structured `careSymbols` array (for
example `[{category: "washing", tempC: 40}, ...]`). Then implement:

- `washing_temperature_valid(temp_c)`: allowed values 30, 40, 60, 70, 95
- `care_treatment_valid(treatment)`: validates against the ISO 3758 symbol set
- a cross-field rule: a declared washing temperature needs a washing method too

---

## Electronics: Regulations (EU) 2023/1670 (ecodesign) and 2023/1669 (energy labelling)

### `electronics/spare_parts.rs`: schema change needed

The electronics schema carries `sparePartsAvailable` as a boolean (or null). There is no
availability *period* field, and period-based rules cannot be checked against a boolean.

Minimum availability periods from earlier ecodesign implementing regulations, for reference:

| Product category | Minimum period | Source |
|---|---|---|
| Washing machines and dryers | 10 years | EU 2019/2022 |
| Dishwashers | 10 years | EU 2019/2022 |
| Refrigerators and freezers | 7 to 10 years | EU 2019/2019 |
| Displays and TVs | 7 to 10 years | EU 2019/2021 |
| Smartphones and laptops | pending | ESPR delegated act not yet set |
| Servers | pending | ESPR delegated act not yet set |
| All other categories | pending | ESPR delegated act not yet set |

**When to update:** when both conditions hold:

1. The electronics schema gains a `sparePartsAvailabilityYears` field or equivalent.
2. An ESPR delegated act sets minimum periods per product category.

Then implement a spare-parts period check here, keyed by the `productCategory` values in
the schema. It returns a typed error enum, not a `String`, so callers can match on the
reason.

---

## Metals

### `metals/aluminium.rs`: CBAM benchmarks implemented, no passport mandate

| Rule | Threshold (kg CO₂e per t) | Status |
|---|---|---|
| Primary route | ≤ 10 000 | CBAM benchmark, **not** a passport mandate |
| Secondary (recycled) route | ≤ 1 000 | CBAM benchmark, **not** a passport mandate |
| Mixed route | ≤ 5 000 | CBAM benchmark, **not** a passport mandate |

CBAM (EU 2023/956) covers embedded-carbon reporting for aluminium imports, but sets no
production-level CO₂e threshold that creates a passport obligation. The thresholds above are
industry and CBAM reference values used by the `product-group-aluminium` plugin.

No delegated act requires an aluminium passport yet. The ESPR Working Plan 2025–2030
(COM(2025) 187) gives aluminium an indicative adoption year of 2027.

`co2e_within_route_threshold` exists so the plugin has one source for the values instead of
hard-coding them. The plugin returns `NOT_ASSESSED`. When a mandate is adopted, update the
constants here and make the plugin return a real determination; the call site stays the same.

**When to update:** when an ESPR delegated act sets mandatory CO₂e thresholds for the
aluminium passport.

---

### `metals/steel.rs`: reference intensities only, no passport mandate

No steel passport mandate is in force. CBAM (EU 2023/956) covers embedded-carbon reporting
for steel imports but sets no per-tonne CO₂e threshold that creates an obligation.

Reference CO₂e intensities per route (worldsteel and IEA figures, not mandated thresholds):

| Route | Typical range (t CO₂e per t steel) |
|---|---|
| `blast-furnace` (BF-BOF) | 1.8 to 2.5 |
| `electric-arc` (EAF, scrap-based) | 0.3 to 0.7 |
| `direct-reduction` (DRI-EAF) | 0.1 to 1.4 |

The `product-group-steel` plugin exists and validates structure and records metrics, but
`dpp-rules` has no steel compliance-checking function yet, only the reference constants
above. The plugin returns `NOT_ASSESSED`, as aluminium's does.

**When to update:** when an ESPR delegated act sets mandatory CO₂e thresholds for the steel
passport. Then implement `co2e_within_route_threshold` here (modelled on the aluminium one)
and switch the plugin from `NOT_ASSESSED` to a real determination. The plugin already exists,
so only the rule function and the plugin's determination call change.

---

## Placeholder product groups

### Construction: `construction/mod.rs`

| Regulation | Status | Passport mandate |
|---|---|---|
| EU CPR 2024/3110 (Construction Products Regulation) | In force | Undated: the delegated act under Art. 75(1) is not adopted, and Art. 22(7) sets the passport date at its entry into force plus 18 months |

No cross-field rules can be implemented until delegated acts under CPR 2024/3110 set passport
field requirements and compliance thresholds per construction product family (cement,
aggregates, structural steel elements and so on).

**When to update:** when the first CPR 2024/3110 delegated act with passport data
requirements is published. Add a sub-module per product family (for example
`construction/concrete.rs`, `construction/structural.rs`), following the battery
sub-modules.

---

### Toys: `toys/mod.rs`

| Regulation | Status | Passport mandate |
|---|---|---|
| EU Toy Safety Regulation 2025/2509 | In force | From 1 August 2030 |

The CE marking check, the one hard rule available today, lives in the `product-group-toy`
plugin rather than in `dpp-rules`, because it is a single-field boolean check JSON Schema can
enforce directly. There are no cross-field rules to implement in `dpp-rules` until the
passport's data requirements are specified.

**When to update:** when the data requirements for the toy passport under Regulation (EU)
2025/2509 are published.

---

## Summary

| Module | Status | Unblocked by |
|---|---|---|
| `batteries/chemistry.rs` | Implemented | |
| `batteries/recycled_content.rs` | Constants and functions ready | The determination switches in 2031, with no code change |
| `batteries/degradation.rs` | Pending | Art. 10(5) delegated acts (due 18 Feb 2026 and 18 Feb 2027) |
| Battery CO₂e class | Pending (belongs in `dpp-calc`) | Art. 7(2) delegated act |
| `textiles/fibre.rs` | Implemented | |
| `textiles/care.rs` | Schema change needed | A structured `careSymbols` field in the textile schema |
| `electronics/spare_parts.rs` | Schema change needed | A period field in the electronics schema and an ESPR delegated act |
| `metals/aluminium.rs` | Benchmarks only | A passport threshold in an ESPR delegated act (indicative adoption 2027) |
| `metals/steel.rs` | Pending | An ESPR delegated act (timeline unknown) |
| `construction/mod.rs` | Not mandated yet | A CPR 2024/3110 Art. 75(1) delegated act (none adopted; undated) |
| `toys/mod.rs` | Not mandated yet | The toy passport data requirements (passport from 1 August 2030) |
