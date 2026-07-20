# Product Lineage — Bill of Materials and Second Life

**Status:** design proposal, not implemented. Supersedes the open questions raised
against the initial BOM/second-life cut.
**Affects:** `dpp-domain` passport model, `dpp-domain::domain::transfer`, engine
verification (`verify_tree`, evidence `componentGraph`).
**Version impact:** breaking — targets the next minor (`0.10.0`), see §8.

Two edges relate one passport to another, and they were shipped as an initial cut
ahead of a requirements pass:

- **`component_refs`** — points *down* to the constituents a product is assembled
  from (its bill of materials).
- **`parent_passport_ref`** — points *up* to the predecessor a second-life unit
  derives from.

This document is that requirements pass. It records what the law actually asks
for, where the current model falls short of it, and the model proposed instead.

---

## 1. What exists today

| Piece | Where | What it does |
|---|---|---|
| `PassportRef` | `dpp-domain::domain::passport::reference` | `uri` + `public_jws_hash` — where to fetch a passport, and a SHA-256 pinning its exact signed public view. Pure data; fetching and checking is the engine's job. |
| `parent_passport_ref: Option<PassportRef>` | `passport.rs` | Upward second-life link. **At most one.** |
| `component_refs: Vec<PassportRef>` | `passport.rs` | Downward BOM links. |
| `TransferRecord` / `TransferChain` | `dpp-domain::domain::transfer` | Dual-signed (outgoing + incoming operator) responsibility handover on **one** passport, with a typed `TransferReason`. |
| `verify_tree` | engine `dpp-vault::domain::verify::tree` | Recursive BOM walk: per-node pin check, depth cap, node cap, path-based cycle detection. Fails closed. |

The primitives are sound. `PassportRef`'s hash-pin is the right idea, and
`verify_tree`'s bounding and cycle handling are careful work. The gaps below are
about *what the edges mean*, not about how they are fetched or checked.

---

## 2. Regulatory requirements

Conventions of `regulatory/COMPLIANCE.md` apply: a claim that cannot be pinned to
the Official Journal text is marked, not asserted.

### 2.1 Second life — Battery Regulation (EU) 2023/1542, Art. 77

The operative sentence, quoted as published:

> For a battery that has been subject to preparation for re-use, preparation for
> repurposing, repurposing or remanufacturing, the responsibility for the
> fulfilment of the obligations under paragraph 4 of this Article shall be
> transferred to the economic operator that has placed that battery on the market
> or has put it into service. Such battery shall have a new battery passport
> linked to the battery passport or passports of the original battery or
> batteries.

Four requirements follow directly from that text:

- **R1 — A second-life unit gets a *new* passport.** Not an edit to the old one.
- **R2 — The new passport links to the passport(s) of the original batter(ies) —
  plural on both sides.** One second-life unit may derive from *several*
  predecessors. This is not a corner case: a stationary storage pack assembled
  from multiple retired EV packs is the canonical second-life product.
- **R3 — Responsibility transfers** to the operator placing the second-life unit
  on the market. The linkage and the responsibility move are one event, not two.
- **R4 — Four distinct operations** are named: *preparation for re-use*,
  *preparation for repurposing*, *repurposing*, *remanufacturing*.

🟠 **COMPLIANCE-PIN PENDING** — the sentence above is verbatim and corroborated
across independent sources, but the paragraph number *within* Art. 77 that carries
it has not been confirmed against the OJ text. Pin it before citing a paragraph in
code.

### 2.2 Change of status — Annex XIII

A battery that has undergone one of the four operations must carry information on
its **change of status**, reported per **point 4 of Annex XIII** and reachable via
the QR code. Reported status values track the product's life (original use,
re-used, repurposed, remanufactured, approaching end of life), and a battery that
becomes waste transfers responsibility again — to the producer or waste-management
operator. A recycled unit's passport is deactivated.

🟠 **COMPLIANCE-PIN PENDING** — the Annex XIII point 4 anchor is corroborated; the
enumerated status values are not yet quoted verbatim from the OJ text. Do not
encode the value list as law until pinned.

### 2.3 ESPR (EU) 2024/1781

ESPR is the framework; it carries no BOM-linkage or second-life-linkage article of
its own. What it does carry, and what therefore still binds every edge here:

- **Art. 9(1)** — passport data "shall be accurate, complete and up to date".
- **Art. 11(1)(e)** — the passport remains available including after insolvency,
  liquidation, or cessation of activity of the responsible operator.

Consequence for lineage: an edge that points at a passport whose operator has
since vanished must still resolve. Per-sector delegated acts are where BOM
granularity will actually be specified; none is in force for our sectors yet, so
**core must not hard-code a sector's notion of "component".**

---

## 3. Gap analysis

### G1 — The model cannot express the plural case (violates R2)

`parent_passport_ref: Option<PassportRef>` holds **one** predecessor. The
regulation says "passport **or passports** of the original battery **or
batteries**". A storage pack built from four retired EV packs cannot be
represented. This is a data-model defect against the plain text, not a missing
nicety.

### G2 — Two mechanisms model one regulatory event (violates R3)

`TransferReason` already carries the Art. 77 vocabulary — `Remanufacturing`,
`Repurposing`, `PreparationForReuse` — but it lives on `TransferRecord`, which
hands responsibility over on a *single, continuing* passport. `parent_passport_ref`
creates a *new* passport but carries no responsibility semantics at all.

R1 and R3 are the same event: a new passport **and** a responsibility move. Today
an operator can perform either half independently, and nothing detects the
inconsistency. Two mechanisms for one event is how they drift.

### G3 — `TransferReason` is missing one of the four operations (violates R4)

Art. 77 names four operations. `TransferReason` has three:
`PreparationForReuse`, `Repurposing`, `Remanufacturing`. **`preparation for
repurposing` is absent.** It is a distinct operation with a distinct actor in the
text.

### G4 — The edge is untyped

`PassportRef` records *where* and *which hash* — never *what relation*. Direction
is encoded in the field name, which was a deliberate simplification and is no
longer sufficient:

- Upward, the four Art. 77 operations have different legal consequences, so the
  edge must say which one occurred.
- Downward, a BOM edge with no quantity or role cannot answer "how much of what,
  where" — the question a BOM exists to answer.

### G5 — No product-life status axis

`PassportStatus` (`Draft`/`Published`/`Suspended`/`Archived`/`Superseded`/
`Deactivated`) is a **publication** lifecycle. Annex XIII point 4 wants a
**product-life** status (original use / re-used / repurposed / remanufactured /
waste). These are orthogonal: a repurposed unit's passport is `Published`. Today
the second axis does not exist, so the change-of-status information has nowhere to
live. `Deactivated` already matches the recycled-unit case and should stay as-is.

### G6 — Neither field is protected from `patch_fields`

The original issue. Neither `parentPassportRef` nor `componentRefs` is in
`PROTECTED_PATCH_FIELDS`, so both are writable through a free-form field patch —
the same bypass class already closed for `operatorIdentifier` and `facility`.

### G7 — A lineage edge is asserted, never consented to

The hash-pin proves the *target* has not been modified. It does not prove the
target's operator agreed to the relationship. Anyone can publish a passport
claiming to derive from, or contain, anyone else's product.

For BOM this is mostly benign (over-claiming a supplier is a commercial problem).
For second life it is not: R3 moves regulatory responsibility, and responsibility
must not be assignable by unilateral assertion.

### G8 — "Component" is undefined across sectors

Left open deliberately, and it should stay open: a battery module, a fibre lot,
and an electronics sub-assembly are not the same kind of thing, and no delegated
act yet defines granularity for our in-force sectors. Core's job is to carry a
pinned reference plus sector-neutral qualifiers, and let sector plugins interpret.

---

## 4. The central question: when may a lineage edge change?

The issue asked whether these fields are create-time-only or attachable
progressively. The answer differs per direction, and follows from a rule the
codebase already enforces elsewhere.

**A field in the signed public view that can change after publish makes the served
body stop verifying against its own signature.** That is the invariant
`AccessPolicy::passport_default()` now states in code, and the reason `lintResult`
was moved off the Public tier.

Both lineage fields are in the signed public view. So:

- **`derived_from` (upward) is create-time by construction.** Per R1 the
  second-life passport *is* the new record; there is no window in which it exists
  without knowing its predecessors.
- **`component_refs` (downward) is create-time, and a BOM change is a new passport
  version.** The mechanism already exists: `supersedes_id` + `version`. Mutating a
  published BOM in place would reintroduce exactly the defect just fixed for
  `lintResult`.

This means **no progressive-attachment port method is needed**, and both fields go
into `PROTECTED_PATCH_FIELDS`. If a sub-assembly's passport genuinely arrives
after publication, the correct response is to supersede with a new version that
includes it — which keeps every signature honest and leaves an auditable trail,
rather than silently rewriting a signed body.

---

## 5. Proposed model

Keep `PassportRef` exactly as it is — a pure "where + pin" primitive, correct and
direction-neutral. Wrap it per direction with the qualifiers each needs.

```rust
/// Upward: a predecessor this unit derives from (Art. 77 second life).
pub struct DerivationRef {
    pub reference: PassportRef,
    pub operation: SecondLifeOperation,
}

/// The four operations named by Art. 77.
pub enum SecondLifeOperation {
    PreparationForReuse,
    PreparationForRepurposing,   // closes G3
    Repurposing,
    Remanufacturing,
}

/// Downward: one constituent in the bill of materials.
pub struct ComponentRef {
    pub reference: PassportRef,
    /// Sector-neutral quantity ("2", "1.4 kg"). Interpreted by sector plugins,
    /// never by core.
    pub quantity: Option<Quantity>,
    /// Sector-defined role of this constituent ("cell", "outer shell").
    pub role: Option<String>,
}
```

On `Passport`:

```rust
// was: parent_passport_ref: Option<PassportRef>
pub derived_from: Vec<DerivationRef>,   // closes G1, G4-up
// was: component_refs: Vec<PassportRef>
pub component_refs: Vec<ComponentRef>,  // closes G4-down
pub life_status: Option<LifeStatus>,    // closes G5 — value list pin-pending
```

### 5.1 Binding lineage to responsibility (closes G2, G3, G7)

The synthesis worth having: **a `TransferRecord` is already dual-signed by both the
outgoing and incoming operator.** That is precisely the consent artefact G7 needs,
and precisely the responsibility move R3 requires.

So bind them rather than adding a third mechanism:

> A passport carrying a non-empty `derived_from` must reference a `TransferRecord`
> for each predecessor, whose `TransferReason` matches that edge's
> `SecondLifeOperation`, and whose incoming operator is this passport's operator.

That single rule closes three gaps at once — the two mechanisms become one event
(G2), the operation vocabulary must agree end-to-end (G3), and a second-life claim
carries the predecessor operator's signature (G7). It is enforceable as a pure
cross-field rule in `dpp-rules`, which is where it belongs.

BOM edges deliberately get **no** consent requirement: it would demand a signature
from every supplier for every assembly, which no supply chain will produce. The
honest position is that a `componentRef` is a *claim by the assembler*, pinned so
it cannot be tampered with, and `verify_tree` already reports exactly that.

### 5.2 What stays out of core

- Any sector's definition of "component" or its granularity (G8).
- Fetching, resolving, and walking edges — engine-side, already correct.
- The Annex XIII status value list, until pinned to the OJ text.

---

## 6. Open questions

1. **Is `life_status` core or sector data?** It is battery-regulation-derived, but
   ESPR delegated acts may generalise it. Recommendation: model it in core as an
   optional, sector-agnostic enum; do not gate compliance on it until pinned.
2. **Does a second-life passport inherit its predecessors' BOM?** Art. 77 requires
   linkage, not re-declaration. Recommendation: no inheritance — linkage is
   sufficient, and copying would duplicate data that can go stale.
3. **What happens to the predecessor's passport?** `Superseded` is wrong (that
   means a new *version* of the same product). A retired EV pack that became part
   of a storage system is not superseded — it is consumed. This may need a status
   value, or may correctly stay `Published` with the derivation edge as the only
   record.
4. **Waste transition.** §2.2 indicates responsibility moves again to the producer
   or waste-management operator. `TransferReason` has no variant for it and
   `PassportStatus::Deactivated` covers only the recycled end state.

---

## 7. Phased plan

| Phase | Scope | Breaking |
|---|---|---|
| **0** | Add `parentPassportRef` + `componentRefs` to `PROTECTED_PATCH_FIELDS` (G6). One line, closes the live bypass, forecloses nothing. | no |
| **1** | Pin Art. 77's paragraph number and the Annex XIII point 4 status list against the OJ text; resolve §6 open questions. | no |
| **2** | `DerivationRef` + `SecondLifeOperation` + plural `derived_from` (G1, G3, G4-up). Add the missing `TransferReason` variant. | **yes** |
| **3** | `ComponentRef` with quantity/role (G4-down); engine `verify_tree` and evidence `componentGraph` follow the new shape. | **yes** |
| **4** | The lineage↔transfer binding rule in `dpp-rules` (G2, G7). | no |
| **5** | `life_status` (G5), gated on Phase 1's pin. | no |

Phase 0 is independently landable and should not wait for the rest.

---

## 8. Compatibility

Phases 2–3 change published field names and types (`parentPassportRef` →
`derivedFrom`; `componentRefs` element type object-ified). Under the lockstep
versioning policy this is a single coordinated `0.10.0` across all core crates,
with the engine following.

Passports already published with the current shape must keep verifying — their
signatures cover the old field names. The read-time upcast lens mechanism
(`?schema_view`) is the existing seam for that and should carry the migration
rather than a one-off compatibility branch.
