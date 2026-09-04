# Product Lineage — Bill of Materials and Second Life

**Status:** Phases 0–5 landed. Phase 5 was non-breaking, and §4.2 records where
it corrected this document.
Supersedes the open questions raised against the initial BOM/second-life cut.
**Affects:** `dpp-domain` passport model, `dpp-domain::transfer`,
`dpp-rules::lineage`, platform-layer verification (`verify_tree`, evidence
`componentGraph`).
**Version impact:** breaking — Phase 2 renamed a published envelope field and
Phase 3 changed another's element type. Both belong to one coordinated minor
bump; see §8, which also corrects a migration mechanism this document previously
named and that does not exist.

Two edges relate one passport to another, and they were shipped as an initial cut
ahead of a requirements pass:

- **`component_refs`** — points *down* to the constituents a product is assembled
  from (its bill of materials).
- **`derived_from`** (was `parent_passport_ref`) — points *up* to the
  predecessors a second-life unit derives from.

This document is that requirements pass. It records what the law actually asks
for, where the current model falls short of it, and the model proposed instead.

---

## 1. What exists today

| Piece | Where | What it does |
|---|---|---|
| `PassportRef` | `dpp-domain::passport::reference` | `uri` + `public_jws_hash` — where to fetch a passport, and a SHA-256 pinning its exact signed public view. Pure data; fetching and checking is the platform layer's job. |
| `derived_from: Vec<DerivationRef>` | `dpp-domain::passport::record` | Upward second-life links, each typed with its Art. 77(7) operation. Plural since Phase 2; was `parent_passport_ref: Option<PassportRef>`, at most one. |
| `component_refs: Vec<ComponentRef>` | `dpp-domain::passport::record` | Downward BOM links, each with an optional quantity and role. Object-ified in Phase 3; was `Vec<PassportRef>`. |
| `TransferRecord` / `TransferChain` | `dpp-domain::transfer` | Responsibility handover on **one** passport, with a typed `TransferReason`. Carries the **outgoing** operator's authorisation plus the hosting node's attestation that acceptance ran — *not* a signature by the incoming operator. See §5.1. |
| `verify_tree` | platform layer | Recursive BOM walk: per-node pin check, depth cap, node cap, path-based cycle detection. Fails closed. |

The primitives are sound. `PassportRef`'s hash-pin is the right idea, and
`verify_tree`'s bounding and cycle handling are careful work. The gaps below are
about *what the edges mean*, not about how they are fetched or checked.

---

## 2. Regulatory requirements

Conventions of `regulatory/COMPLIANCE.md` apply: a claim that cannot be pinned to
the Official Journal text is marked, not asserted.

### 2.1 Second life — Battery Regulation (EU) 2023/1542, Art. 77(7)

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

✅ **COMPLIANCE-PIN: EU 2023/1542, Art. 77(7) (OJ L 191, 28.7.2023, p. 73).**
Read directly from the Official Journal text. The quoted sentence is the first
subparagraph of **paragraph 7**, and the "obligations under paragraph 4" it
transfers are Art. 77(4)'s duty to keep passport information "accurate, complete
and up to date". The prior 🟠 residual — the paragraph number within Art. 77 — is
closed.

The **second subparagraph of the same paragraph** carries the waste transition,
and it answers §6 question 4 directly:

> Where the status of a battery changes to that of a waste battery, the
> responsibility for the fulfilment of the obligations under paragraph 4 of this
> Article shall be transferred either to the producer or, where appointed in
> accordance with Article 57(1), the producer responsibility organisation, or the
> waste management operator selected in accordance with Article 57(8).

- **R5 — The waste transition moves responsibility again**, to one of three named
  recipients, and — unlike the first subparagraph — it mandates **no new
  passport**. Responsibility moves while the record stays. See §4.

**Art. 77(8)** gives the only termination the article states: "A battery passport
shall cease to exist after the battery has been recycled." Recycling, not
consumption by a second-life unit, is what ends a passport. This answers §6
question 3.

The four operations are defined terms, and the boundary between the two
repurposing ones is the **waste status of the input**, not the actor:

- **Art. 3(30)** — *preparation for repurposing*: "any operation, by which a
  **waste battery**, or parts thereof, is prepared so that it can be used for a
  different purpose or application than that for which it was originally
  designed".
- **Art. 3(31)** — *repurposing*: "any operation that results in a battery, **that
  is not a waste battery**, or parts thereof being used for a purpose or
  application other than that for which the battery was originally designed".

✅ **COMPLIANCE-PIN: EU 2023/1542, Art. 3(29)–(32) (OJ L 191, 28.7.2023, p. 27).**

### 2.2 Change of status — Annex XIII point 4(c)

A battery that has undergone one of the four operations must carry information on
its **change of status**, reported per **point 4 of Annex XIII** and reachable via
the QR code. The enumerated values, quoted as published:

> (c) information on the status of the battery, defined as 'original',
> 'repurposed', 're-used', 'remanufactured' or 'waste';

✅ **COMPLIANCE-PIN: EU 2023/1542, Annex XIII point 4(c) (OJ L 191, 28.7.2023,
p. 109).** Read directly from the Official Journal text.

**The value list this document previously carried was wrong in two of five**, which
is exactly what the pin existed to catch. It read *"original use, re-used,
repurposed, remanufactured, approaching end of life"*. The OJ says `'original'`,
not "original use"; and **"approaching end of life" is not a status value in this
Regulation at all** — the string does not occur anywhere in its text, and the fifth
value is `'waste'`. Encoding the unpinned list would have shipped a status value
that does not exist in EU law.

Two further facts follow from where point 4 sits, and both bind Phase 5:

- **Point 4 is the legitimate-interest tier.** Its heading is "INFORMATION AND DATA
  RELATING TO AN INDIVIDUAL BATTERY ACCESSIBLE ONLY TO PERSONS WITH A LEGITIMATE
  INTEREST". `life_status` is therefore **not public**. `Disclosure::Individual`
  already carries exactly this meaning — its own doc names "status" — so the field
  must be classified there rather than defaulting to public.
- **Status is reported on change, over the life of one record.** Point 4(a) requires
  the performance and durability values "when the battery is placed on the market
  **and when it is subject to changes in its status**". A status that is reported on
  change is not a create-time constant. See §4.

### 2.3 ESPR (EU) 2024/1781

ESPR is the framework; it carries no BOM-linkage or second-life-linkage article of
its own. What it does carry, and what therefore still binds every edge here:

- **Art. 9(1)** — passport data "shall be accurate, complete and up to date".
- **Art. 11(1)(e)** — the passport remains available including after insolvency,
  liquidation, or cessation of activity of the responsible operator.

Consequence for lineage: an edge that points at a passport whose operator has
since vanished must still resolve. Per-product group delegated acts are where BOM
granularity will actually be specified; none is in force for our product groups yet, so
**core must not hard-code a product group's notion of "component".**

---

## 3. Gap analysis

### G1 — The model could not express the plural case (violated R2) — **CLOSED**

`parent_passport_ref: Option<PassportRef>` held **one** predecessor. The
regulation says "passport **or passports** of the original battery **or
batteries**". A storage pack built from four retired EV packs could not be
represented. This was a data-model defect against the plain text, not a missing
nicety.

Closed in Phase 2: `derived_from: Vec<DerivationRef>`.

### G2 — Two mechanisms modelled one regulatory event (violated R3) — **CLOSED**

`TransferReason` carries the full Art. 77(7) vocabulary since Phase 1 — all four
operations — but it lives on `TransferRecord`, which hands responsibility over on
a *single, continuing* passport. `derived_from` creates a *new* passport and names
the operation. The two vocabularies agreed and nothing required them to.

R1 and R3 are the same event: a new passport **and** a responsibility move. An
operator could perform either half independently and nothing detected the
inconsistency. Two mechanisms for one event is how they drift.

Closed in Phase 4: `dpp_rules::lineage::consent` requires them to agree, so the
two mechanisms now describe one event or the passport is reported as
unsupported.

### G3 — `TransferReason` was missing one of the four operations (violated R4) — **CLOSED**

Art. 77(7) names four operations. `TransferReason` had three:
`PreparationForReuse`, `Repurposing`, `Remanufacturing`. **`preparation for
repurposing` was absent**, so a transfer performed for that reason could only be
recorded as one of the other three.

An earlier draft of this section said the distinction was "a distinct actor in
the text". That is not what the text says, and the correction matters for anyone
implementing the boundary: **Art. 3(30) and Art. 3(31) separate the two by the
waste status of the input** — preparation for repurposing operates on a *waste*
battery, repurposing on one that is *not* a waste battery. Both may be performed
by the same actor.

Closed ahead of Phase 2: adding a variant to `TransferReason` is additive, and the
enum is `#[non_exhaustive]`, so it needs neither Phase 2's `SecondLifeOperation`
nor a breaking release. It does **not** resolve the separate open question in
`docs/regulatory/COMPLIANCE.md` about whether the second-life operations belong on
`TransferReason` at all — it makes the set consistent with the article it is drawn
from, so that if those variants are later withdrawn, all four go together.

### G4 — The edge was untyped — **CLOSED**

`PassportRef` records *where* and *which hash* — never *what relation*. Direction
was encoded in the field name, which was a deliberate simplification and is no
longer sufficient:

- Upward, the four Art. 77(7) operations have different legal consequences, so
  the edge must say which one occurred. **Closed in Phase 2** — `DerivationRef`
  carries a required `SecondLifeOperation`.
- Downward, a BOM edge with no quantity or role cannot answer "how much of what,
  where" — the question a BOM exists to answer. **Closed in Phase 3** —
  `ComponentRef` carries an optional `quantity` and `role`, both of which core
  transports and never interprets (G8).

### G5 — No product-life status axis

`PassportStatus` (`Draft`/`Published`/`Suspended`/`Archived`/`Superseded`/
`Deactivated`) is a **publication** lifecycle. Annex XIII point 4(c) wants a
**product-life** status — `original` / `repurposed` / `re-used` /
`remanufactured` / `waste`, now pinned in §2.2. These are orthogonal: a repurposed
unit's passport is `Published`. Today the second axis does not exist, so the
change-of-status information has nowhere to live. `Deactivated` matches Art.
77(8)'s post-recycling end state and should stay as-is.

### G6 — Neither field was protected from `patch_fields` — **CLOSED**

The original issue. Neither `parentPassportRef` nor `componentRefs` was in
`PROTECTED_PATCH_FIELDS`, so both were writable through a free-form field patch —
the same bypass class already closed for `operatorIdentifier` and `facility`.
Both are in the list now (Phase 0). What that protection may and may not be read
to mean is §4.

### G7 — A lineage edge was asserted, never consented to — **CLOSED for second life**

The hash-pin proves the *target* has not been modified. It does not prove the
target's operator agreed to the relationship. Anyone can publish a passport
claiming to derive from, or contain, anyone else's product.

For BOM this is mostly benign (over-claiming a supplier is a commercial problem),
and it stays that way on purpose — see §5.1. For second life it is not: R3 moves
regulatory responsibility, and responsibility must not be assignable by
unilateral assertion.

Closed in Phase 4 for the derivation direction: `dpp_rules::lineage::consent`
refuses a second-life edge that no transfer of responsibility supports. The
consent it rests on is the **outgoing** operator's authorisation — the
predecessor's own — which §5.1 explains, and which is not the reason this
document originally gave.

### G8 — "Component" is undefined across product groups

Left open deliberately, and it should stay open: a battery module, a fibre lot,
and an electronics sub-assembly are not the same kind of thing, and no delegated
act yet defines granularity for our in-force product groups. Core's job is to carry a
pinned reference plus product group-neutral qualifiers, and let product group plugins interpret.

---

## 4. The central question: when may a lineage edge change?

The issue asked whether these fields are create-time-only or attachable
progressively. The answer follows from a rule the codebase already enforces
elsewhere.

**A field in the signed public view that can change after publish makes the served
body stop verifying against its own signature.** That is the invariant
`AccessPolicy::passport_default()` now states in code, and the reason `lintResult`
was moved off the Public tier.

Both lineage fields are in the signed public view. So:

- **`derived_from` (upward) is fixed once signed.** Per R1 the second-life passport
  *is* the new record, and its predecessors are known when it is created.
- **`component_refs` (downward) is fixed once signed, and a BOM change on a
  published passport is a new passport version.** The mechanism already exists:
  `supersedes_id` + `version`. Mutating a published BOM in place would reintroduce
  exactly the defect just fixed for `lintResult`.

**No progressive-attachment port method is needed**, and both fields are in
`PROTECTED_PATCH_FIELDS`. If a sub-assembly's passport genuinely arrives after
publication, the correct response is to supersede with a new version that includes
it — which keeps every signature honest and leaves an auditable trail, rather than
silently rewriting a signed body.

### 4.1 The invariant is *immutable once signed*, not *create-time*

An earlier draft of this section derived the rule from the signed public view and
then generalised it to "create-time by construction". Those are not the same
claim, and the stronger one is wrong.

**The draft window is legitimate.** Between create and first publish there is no
signature to break, and assembling a bill of materials incrementally before first
publish is the normal case, not an edge one. The only shipping backend already
depends on this: it derives its protected list from `PROTECTED_PATCH_FIELDS` with
`componentRefs` removed, precisely because the update path it serves accepts
drafts only. This section as previously written forbade what that backend does,
which is the wrong way round — the backend is right and the rule was overstated.

**So the exception belongs in the rule, with its condition attached.** An
implementation should be able to say *"editable while `Draft`"* rather than
*"not protected"*. Stated as an absence from a list, the exception is
unconditional at the trait boundary; stated as a condition, it survives contact
with a caller that did not expect it.

**And that distinction is load-bearing, because the guard is not where the
exception is.** `patch_fields` checks passport status on neither side — the
draft-only check lives in the *service that calls it*. There is already a second
caller that reaches `patch_fields` with **published** rows: the lint re-check,
which persists `lintResult` on every published passport. Nothing is broken today,
because that caller patches one field and does not touch lineage. But what makes a
third caller safe is this crate's list, and `componentRefs` is exactly the entry a
backend removes from it. The invariant then rests on every current and future
caller re-checking status — the same shape as the drift that put
`operatorIdentifier`, `facility` and `parentPassportRef` at risk before.

Any second implementation will face the same choice with no guidance and no reason
to make it the same way. That is what this section has to fix before Phase 2
rewrites these fields.

### 4.2 `life_status` is not governed by this rule at all

Phase 5's field is a different case, and the rule above must not be extended to it
by analogy.

Per §2.2 it is `Disclosure::Individual`, so it is not in the signed *public* view;
and per Art. 77(7) second subparagraph the **waste transition mandates no new
passport**. A battery whose status changes to waste moves responsibility while
keeping its record — and Annex XIII point 4(a) expects values reported "when it is
subject to changes in its status". A create-time-only `life_status` could never
reach `'waste'`, which is one of the five values the law enumerates.

The four Art. 77(7) operations *do* each produce a new passport (R1), so those four
values are set at create. `'waste'` is the transition that is not, and it is the
reason `life_status` needs a defined mutation path. It is not a free patch field
either, since the transition is also a responsibility move under R5.

**Phase 5 specified that path: the waste transition is a new passport version.**
An earlier draft of this section expected the path to be an alternative *to*
`PROTECTED_PATCH_FIELDS`. It is the opposite — the path is `supersedes_id` +
`version`, which is §4's existing answer to "a signed field must change", and
being in `PROTECTED_PATCH_FIELDS` is precisely what forces a caller onto it.
`life_status` is therefore in that list with the other signed fields.

The alternative was a dedicated port method. It does not survive contact with
the rest of the design: it would break the served body's signature and have to
re-sign, which is a version bump wearing a disguise, and it would introduce a
second mutation mechanism for the one field that least needs one. Versioning
also leaves the audit trail a responsibility move under R5 ought to leave.

`PassportStatus::Superseded` is the right publication status for the old version
here — it genuinely is a new version of the same product. That is distinct from
a predecessor *consumed* by a second-life unit, which stays `Published`, since
Art. 77(8) makes recycling the only termination.

---

## 5. Proposed model

**Both edge types shipped** — `DerivationRef` and `SecondLifeOperation` in
Phase 2, `ComponentRef` and `Quantity` in Phase 3, all in `dpp-domain::passport`.
`LifeStatus` shipped in Phase 5, alongside `TransferReason::WasteHandover` and
`dpp-rules::lineage::check_life_status_consistency`.

Keep `PassportRef` exactly as it is — a pure "where + pin" primitive, correct and
direction-neutral. Wrap it per direction with the qualifiers each needs.

```rust
/// Upward: a predecessor this unit derives from (Art. 77(7) second life).
pub struct DerivationRef {
    pub reference: PassportRef,
    pub operation: SecondLifeOperation,
}

/// The four operations named by Art. 77(7). Mirrors the `TransferReason`
/// variants of the same names, which §5.1 requires it to agree with.
pub enum SecondLifeOperation {
    PreparationForReuse,
    PreparationForRepurposing,
    Repurposing,
    Remanufacturing,
}

/// Product-life status, Annex XIII point 4(c). The five values are the
/// enumerated list, pinned in §2.2 — not a superset, and not renamed.
///
/// Classified `Disclosure::Individual`: point 4 is the legitimate-interest
/// tier, so this must not default to the public view.
pub enum LifeStatus {
    Original,
    Repurposed,
    Reused,
    Remanufactured,
    Waste,
}

/// Downward: one constituent in the bill of materials.
pub struct ComponentRef {
    pub reference: PassportRef,
    /// Product group-neutral quantity ("2", "1.4 kg"). Interpreted by product group plugins,
    /// never by core.
    pub quantity: Option<Quantity>,
    /// Product group-defined role of this constituent ("cell", "outer shell").
    pub role: Option<String>,
}

/// `Quantity` was named here but never defined. As shipped:
/// `unit: None` means a dimensionless count — two of a thing, not two
/// kilograms of it. `f64` matches every other physical quantity in the crate
/// (`MaterialEntry::weight_kg`); core does no arithmetic on it. The unit is an
/// opaque label, not a vocabulary core validates against — that would be core
/// deciding a product group's semantics, which G8 forbids.
pub struct Quantity {
    pub value: f64,
    pub unit: Option<String>,
}
```

On `Passport`:

```rust
// was: parent_passport_ref: Option<PassportRef>
pub derived_from: Vec<DerivationRef>,   // closes G1, G4-up
// was: component_refs: Vec<PassportRef>
pub component_refs: Vec<ComponentRef>,  // closes G4-down
pub life_status: Option<LifeStatus>,    // closes G5 — value list now pinned
```

`life_status` is `Option` because Annex XIII point 4 is battery-regulation
content and no other in-force product group requires it; `None` means "this
product group does not report a product-life status", which is distinct from
`Some(Original)`.

### 5.1 Binding lineage to responsibility (closes G2, G7)

The synthesis worth having: **a second-life operation already *is* a transfer of
responsibility under Art. 77(7)**, and a transfer record already carries the
outgoing operator's own authorisation. That is the consent artefact G7 needs, and
the responsibility move R3 requires, in one artefact that exists.

🚨 **An earlier version of this section justified that differently, and was
wrong.** It said a `TransferRecord` is "dual-signed by both the outgoing and
incoming operator". It is not, and `TransferRecord` says so at the field itself:
the acceptance half is the **hosting node's attestation that the acceptance step
ran**, not a signature by the incoming operator, who has no key on the node
recording it. That field was renamed from `to_signature` precisely because the
old name invited this misreading, and this document then made it anyway.

The conclusion survives, for a reason worth stating because it is easy to get
backwards. **The consent this rule needs is the predecessor's**, and in a
second-life transfer the predecessor's operator is the *outgoing* party — so the
half that is a genuine party signature is exactly the half that matters. The
incoming half would have been this passport's own operator asserting something
about itself, which was never evidence of anything. Acceptance is still checked,
because an abandoned or rejected handover should not support a live claim, but it
is checked as a *status*, not as a second signature.

So bind them rather than adding a third mechanism:

> A passport carrying a non-empty `derived_from` must reference a `TransferRecord`
> for each predecessor, whose `TransferReason` matches that edge's
> `SecondLifeOperation`, and whose incoming operator is this passport's operator.

That single rule closes two gaps at once — the two mechanisms become one event
(G2), and a second-life claim carries the predecessor operator's authorisation
(G7). It shipped in Phase 4 as `dpp_rules::lineage::consent`, a pure cross-field
rule, which is where it belongs. It also depended on G3 being closed first: the
rule matches a `SecondLifeOperation` against a `TransferReason`, so a
`TransferReason` that could not express `preparation for repurposing` would have
made the rule unsatisfiable for one of the four operations.

**One thing the rule cannot do, and does not pretend to.** It cannot tell whether
a transfer record actually concerns the predecessor a given edge points at: an
edge identifies its target by URI, a transfer identifies its subject by passport
id, and resolving one to the other is a network fetch. So the **caller
correlates and the rule checks**. An edge whose caller found no corresponding
transfer is reported as unconsented — the correct reading of "no evidence was
found", rather than an assumption that none exists. That also means the rule is
enforced platform-side, because core cannot resolve a URI.

BOM edges deliberately get **no** consent requirement: it would demand a signature
from every supplier for every assembly, which no supply chain will produce. The
honest position is that a `componentRef` is a *claim by the assembler*, pinned so
it cannot be tampered with, and `verify_tree` already reports exactly that.

### 5.2 What stays out of core

- Any product group's definition of "component" or its granularity (G8).
- Fetching, resolving, and walking edges — platform-side, already correct.
- Any status value beyond the five Annex XIII point 4(c) enumerates. The list is
  now pinned (§2.2) and is closed: a sixth value would be an invention, which is
  what the unpinned draft of this document produced.

---

## 6. Questions resolved

All four are settled. Two were answerable from the OJ text and are now pinned;
two were design calls and are recorded as decided, not as recommendations.

1. **Is `life_status` core or product group data? — Core, optional, not
   compliance-gating.** It is battery-regulation-derived, but ESPR delegated acts
   may generalise it, and the shape (a small closed enum on the passport envelope)
   is product-group-neutral even though the obligation is not. Modelled in core as
   `Option<LifeStatus>`; no rule gates on it. Its disclosure class is **not** a
   design call — Annex XIII point 4 fixes it at `Disclosure::Individual` (§2.2).

2. **Does a second-life passport inherit its predecessors' BOM? — No.** Art. 77(7)
   requires the new passport to be *linked* to the predecessors' passports; it
   requires no re-declaration of their contents. Copying would duplicate data that
   can go stale, and the predecessors keep their own passports (question 3), so the
   data remains reachable through the edge.

3. **What happens to the predecessor's passport? — It stays `Published`.**
   Answered by the text rather than by taste: **Art. 77(8)** gives the only
   termination — "A battery passport shall cease to exist after the battery has
   been recycled." Being consumed by a second-life unit is not recycling, so no
   termination is triggered. `Superseded` remains wrong for the reason already
   stated (it means a new *version* of the same product), and no new
   `PassportStatus` value is needed. The derivation edge is the record of what
   happened, which is exactly what Art. 77(7) asks for.

4. **Waste transition. — Resolved by Art. 77(7) second subparagraph** (quoted in
   §2.1), which was not read when this question was written. Responsibility moves
   to one of three named recipients: the producer, the producer responsibility
   organisation where appointed under Art. 57(1), or the waste management operator
   selected under Art. 57(8). Two consequences:
   - It mandates **no new passport**, so the record continues and its
     `life_status` becomes `'waste'` in place. That is why §4.2 exempts
     `life_status` from the immutable-once-signed rule.
   - `TransferReason` still has no variant for it, and it is a genuine transfer of
     responsibility under this subparagraph. Adding one is **Phase 5's** work, not
     Phase 2's, because the recipient set is the waste-operator vocabulary rather
     than the second-life one. `PassportStatus::Deactivated` continues to cover
     only the Art. 77(8) post-recycling end state, which is a later and distinct
     event.

---

## 7. Phased plan

| Phase | Scope | Breaking | State |
|---|---|---|---|
| **0** | Add `parentPassportRef` + `componentRefs` to `PROTECTED_PATCH_FIELDS` (G6). One line, closes the live bypass, forecloses nothing. | no | **landed** |
| **1** | Pin Art. 77(7), Art. 3(29)–(32) and the Annex XIII point 4(c) status list against the OJ text; reconcile §4; resolve the §6 questions; add the `TransferReason` variant (G3). | no | **landed** |
| **2** | `DerivationRef` + `SecondLifeOperation` + plural `derived_from` (G1, G4-up). | **yes** | **landed** |
| **3** | `ComponentRef` with quantity/role (G4-down). The verification walk and the evidence component graph follow the new shape platform-side. | **yes** | **landed** |
| **4** | The lineage↔transfer binding rule in `dpp-rules` (G2, G7). Enforced platform-side, since correlating an edge to a transfer needs a URI resolved. | no | **landed** |
| **5** | `life_status` (G5) with the §2.2 value list and `Disclosure::Individual`; its mutation path per §4.2; the waste `TransferReason` variant (§6 question 4). | no | proposal |

Phase 0 was independently landable and did not wait for the rest. Phase 1 carried
G3 as well: the variant is additive on a `#[non_exhaustive]` enum, so it needed
neither Phase 2's breaking release nor `SecondLifeOperation` to exist first, and
Phase 4's binding rule is unsatisfiable without it.

Phases 2 and 3 are independent of each other; 4 needs 2's vocabulary; 5 is
unblocked by Phase 1's pin and independent of 2–4.

---

## 8. Compatibility

Phases 2 and 3 changed published field names and types (`parentPassportRef` →
`derivedFrom`; `componentRefs` element type object-ified). Both are done, and
under the lockstep versioning policy they are one coordinated minor version bump
across all core crates, with the platform layer following — **they must ship in
the same release**, which is the whole reason they were sequenced together.

🚨 **An earlier version of this section named a mechanism that does not exist.**
It said the read-time upcast lens (`?schema_view`) "is the existing seam" and
"should carry the migration". It cannot. `Passport::from_stored` lenses
`productGroupData` and nothing else, and both it and `Passport::schema_version`
say so explicitly: **envelope fields are never lensed, deliberately**, because a
lens transforms one product group's sub-object while an envelope field is shared
by every product group's documents, so getting an envelope transform wrong
corrupts all of them at once.

The envelope's actual rule is additive-only — `Option<T>` + `#[serde(default)]`,
or a rename that keeps accepting the old key. **Phase 2 did not do that**, and
the decision was taken knowingly: this project has no published passports to
strand, so the cost is currently zero. Two things follow, and neither should be
inherited by Phase 3 without a fresh decision:

- **The failure was silent, and is now loud.** `Passport` sets no
  `deny_unknown_fields` and `derived_from` defaults, so a document still carrying
  `parentPassportRef` deserialized *successfully* and arrived with no lineage
  edge — quieter, and worse, than the `sector` → `productGroup` rename, where the
  renamed field was required and pre-rename documents refused to load.
  `REMOVED_ENVELOPE_KEYS` now names the old key and `Passport::from_stored`
  refuses any document carrying one. **Phase 3 should add its own entry there if
  it removes a key**; changing an element *type* fails loudly on its own, because
  `#[serde(default)]` applies only when the key is absent.
- **The licence expires when the first real passport is published.** After that,
  an envelope rename must carry the old key or a one-time document rewrite in
  the publish pipeline. A signature covers the old key names, so a rewritten
  document does not re-verify and a silently-emptied one verifies while being
  wrong.

Phase 3 object-ified `componentRefs` elements, and that is a different failure
again. The key name did not change, and `#[serde(default)]` applies only when a
key is *absent* — so a document carrying the old element shape would fail
deserialization on its own, loudly, needing no `REMOVED_ENVELOPE_KEYS` entry.

**Loud is the wrong answer here, and that is the more important lesson.** The
first analysis of this asked only where a document is *stored*. It is also
**fetched**: `componentRefs` is in the signed public view, and other operators'
nodes read it over the network to verify a bill of materials. A fetched passport
is signed and belongs to someone else — it can never be rewritten into the new
shape, by anyone. Refusing it refuses data that is correct, current and
unforgeable, permanently.

And refusal does not degrade gracefully. A verification walk that cannot parse an
entry reports a malformed reference, which is graded as an **integrity
violation**, in the same class as a hash mismatch or a cycle. So an upgraded node
would report a not-yet-upgraded node's bill of materials as *tampered*, on
evidence that is nothing but a version difference. Nodes are independent
per-operator deployments, so that skew is the steady state, not a window.

`ComponentRef` therefore hand-writes `Deserialize` and accepts both shapes. This
is the same *"a rename that keeps accepting the old key"* discipline the envelope
rule already states, applied to an element type instead of a key name.

**The distinction to carry forward, in full:**

| Change | Stored document | Fetched document |
|---|---|---|
| Rename a key | silent — needs a tripwire | silent lineage loss |
| Change a present key's type | loud — self-reporting | **loud, and reads as tampering** |

A field in the signed public view answers to the second column. Deciding its
compatibility from the first alone is how this was nearly shipped wrong.

**Phase 2 is not fully covered by this.** `derivedFrom`'s rename degrades
silently across nodes rather than accusing anyone, which is milder — but the GS1
resolver reads `parentPassportRef` from fetched JSON to build predecessor link
relations, so a version-skewed node emits none. That is a deliberate open
decision, not an oversight.
