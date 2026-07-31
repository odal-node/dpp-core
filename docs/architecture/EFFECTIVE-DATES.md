# Regulatory effective dates

How `dpp-calc` decides which law governs a product, and the one piece that is
deliberately not built yet.

**Status:** current as of 2026-07-25. The verified regulatory research and
citations behind these dates are held separately; this document covers the
*model*, not the facts it encodes.

---

## 1. Two dates, never one

`clock::AssessmentClock` carries both and keeps them apart:

| Field | What it is | Where it comes from |
|---|---|---|
| `law_in_force_on` | The date the governing law attached to the product | The product's own record — for batteries, `placedOnMarketDate` |
| `computed_at` | When the calculation ran | The wall clock, and nothing else |

EU staged obligations attach at a regulated triggering event. Regulation (EU)
2023/1542 Art. 8(2) binds *"industrial batteries … placed on the market from
18 August 2031"* — so a battery lawfully placed on the market in 2030 never
acquires those minimums, however late it is audited. Art. 10(4) says the same
thing from the other direction, disapplying the performance duties to batteries
placed on the market before those duties applied.

There is no `AssessmentClock::now()`, and the calculators have no wall-clock
overload. Both omissions are deliberate: a convenience constructor that reads
the clock makes the wrong answer the shorter one to write, and the wrong answer
here is silent — it produces correct-looking results until a phase boundary
passes, then produces retroactively wrong ones.

`CalculationReceipt.assessed_as_of` records `law_in_force_on`. Without it a
receipt names the ruleset it cited but not whether that was the right ruleset to
cite, because selection is a function of this date.

## 2. Effectivity: in force, or undated

`ruleset::Effectivity` has two variants:

```rust
InForce { from: NaiveDate, until: Option<NaiveDate> }
Pending { empowerment: &'static str, adoption_deadline: Option<NaiveDate> }
```

`Pending` is not "in force from a distant date". It governs **no date at all**,
including far-future ones, and `ensure_active_on` returns
`CalcError::RulesetUndetermined` naming the instrument being waited on.

An earlier design used `from = 2100-01-01` as a pending sentinel. That asserted a
calendar date for an act that does not exist — a fact the regulation does not
state — and it made "pending" indistinguishable from "starts a long time from
now". It also forced a second, hand-maintained copy of the same fact in the
calculator status map, free to drift from the rulesets themselves.
`SectorCalculatorEntry::status_on` now derives status from `Effectivity`.

`adoption_deadline` is the Commission's own deadline **from the regulation
text**, where one exists. It is not an application date: several have already
passed unmet. Never populate it with an estimated or expected year — an
approximate guess in this field would read as a legal fact.

## 3. Assessability: four reasons, not one blank

`assessability::Assessability<T>` replaces `Option` on `resolve_*`:

| Variant | What to tell an operator |
|---|---|
| `Assessed(T)` | — |
| `NotYetInForce { applies_from }` | Covered, from a date we can name |
| `Undetermined { empowerment }` | Covered in principle, awaiting an unadopted act |
| `Expired { until, superseded_by }` | The governing rule was replaced — follow the successor |
| `OutOfScope` | This regulation does not cover the product |

None of these is non-compliance, and collapsing them into `None` meant a caller
could not distinguish "we do not assess this" from "you fail this". Where several
rulesets exist for one category and none is active, the reason comes from the row
closest to applying: a known start date, then an undated pending act, then an
expired rule.

---

## 4. Not built: conditional application dates

**This is the known gap. It is deferred on purpose, not overlooked.**

### The problem

EU staged obligations are routinely written as a *maximum of two or more terms*,
not a fixed date. Verbatim, Reg. (EU) 2023/1542 Art. 7(1):

> The carbon footprint declaration shall apply from:
> (a) 18 February 2025 **or 12 months after the date of entry into force either
> of the delegated act or of the implementing act** respectively referred to in
> the fourth subparagraph, points (a) and (b), **whichever is the latest**, for
> electric vehicle batteries; …

So:

```
applies_from = max(
    floor_date,
    entry_into_force(delegated_act)   + offset_months,
    entry_into_force(implementing_act) + offset_months,
)
```

Three properties the current model cannot express:

1. **Two instruments, not one.** Art. 7 depends on a *delegated* act (methodology)
   and an *implementing* act (declaration format). The trigger is whichever
   enters into force later.
2. **The offset differs per obligation.** 12 months for Art. 7(1) EV batteries,
   18 months for most others, 24 months for Art. 8(1).
3. **The floor is per battery category.** Art. 7 alone has three obligations
   × four categories = twelve distinct floor dates.

Known instances: Art. 7(1), 7(2) and 7(3) (twelve date ladders in total),
Art. 8(1) (two), Art. 10(2) and 10(3) (two).

### Why it is deferred

**No relevant instrument has entered into force.** Every trigger is currently
unknown, so the arithmetic has nothing real to compute against and could only be
validated against fixtures we invented. Building a date calculator whose inputs
are all imaginary is the same mistake as the `A–E` carbon-class enumeration:
modelling a shape the law has not published.

`Pending` already gives the correct *answer* today — "this cannot be determined,
here is the empowerment we are waiting on". The arithmetic changes how precisely
we can answer once an act lands, not whether today's answer is right.

### What to build, when the first act lands

Extend `Effectivity` with a third variant rather than mutating `Pending`:

```rust
Conditional {
    /// The floor in the regulation text ("From 18 February 2026 …").
    not_before: NaiveDate,
    /// "…or N months after the date of entry into force of…"
    offset_months: u32,
    /// All instruments whose entry into force starts the offset. The latest wins.
    triggers: &'static [Trigger],
}

struct Trigger {
    empowerment: &'static str,
    kind: InstrumentKind,              // Delegated | Implementing
    adoption_deadline: Option<NaiveDate>,
    /// `None` until the act is adopted and its OJ entry-into-force date is read.
    entered_into_force: Option<NaiveDate>,
}
```

`Conditional` resolves to `InForce` once every trigger has an
`entered_into_force`, and behaves like `Pending` until then — so
`Assessability::Undetermined` remains the answer without any caller changing.

Two rules for whoever picks this up:

- **`entered_into_force` is read from the Official Journal text, never inferred**
  from an adoption date or a press release. Entry into force is usually the
  twentieth day following publication, but the act states its own rule and some
  differ — Reg. (EU) 2025/1561 entered into force the day after publication.
- **Add a golden vector per obligation ladder**, asserting the computed
  `applies_from` against the date stated in the act's own transitional provision.
  If the two disagree, the model is wrong, not the act.

### Triggers to watch

The instruments that would first make this worth building, with their deadlines
from the regulation text (all unadopted as of 2026-07-25):

| Empowerment | Deadline | Starts |
|---|---|---|
| Art. 8(1) 3rd sub — recycled content methodology | 18 Aug 2026 | Art. 8(1) declaration duty |
| Art. 7(2) 4th sub (a) — CF performance classes, industrial | 18 Aug 2026 | Art. 7(2) class + label |
| Art. 10(5) 1st sub — minimum performance values, industrial | 18 Feb 2026 (passed) | Art. 10(2) |
| Art. 77(9) — passport access rights | 18 Aug 2026 | Art. 78(b), (f) |
