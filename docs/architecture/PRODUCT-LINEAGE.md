# Product lineage: bill of materials and second life

<!-- BOUNDARY-EXCEPTION(taxonomy): the phases are this document's own plan, defined in §7, not a roadmap outside it. -->

**Status:** Phases 0 to 5 have landed. Phase 5 was non-breaking, and §4.2 records where it
corrected this document. It supersedes the open questions raised against the initial
bill-of-materials and second-life design.
**Affects:** the `dpp-domain` passport model, `dpp-domain::transfer`, `dpp-rules::lineage`,
and host-side verification (a recursive bill-of-materials walk and an evidence component
graph).
**Version impact:** breaking. Phase 2 renamed a published envelope field and Phase 3
changed another's element type. Both belong to one coordinated minor bump; see §8, which
also corrects a migration mechanism this document previously named and that does not
exist.

Two edges relate one passport to another, and they shipped as an initial design ahead of
a requirements pass:

- **`component_refs`** points *down* to the constituents a product is assembled from (its
  bill of materials).
- **`derived_from`** (formerly `parent_passport_ref`) points *up* to the predecessors a
  second-life unit derives from.

This document is that requirements pass. It records what the law asks for, where the
earlier model fell short, and the model adopted instead.

---

## 1. What exists today

| Piece | Where | What it does |
|---|---|---|
| `PassportRef` | `dpp-domain::passport::reference` | `uri` plus `public_jws_hash`: where to fetch a passport, and a SHA-256 pinning its exact signed public view. Plain data; fetching and checking belong to your application. |
| `derived_from: Vec<DerivationRef>` | `dpp-domain::passport::record` | Upward second-life links, each typed with its Art. 77(7) operation. Plural since Phase 2; was `parent_passport_ref: Option<PassportRef>`, at most one. |
| `component_refs: Vec<ComponentRef>` | `dpp-domain::passport::record` | Downward bill-of-materials links, each with an optional quantity and role. Objects since Phase 3; was `Vec<PassportRef>`. |
| `life_status: Option<LifeStatus>` | `dpp-domain::passport::record` | The Annex XIII point 4(c) product-life status. Phase 5. |
| `TransferRecord` / `TransferChain` | `dpp-domain::transfer` | Responsibility handover on **one** passport, with a typed `TransferReason`. Carries the **outgoing** operator's authorisation plus the hosting node's attestation that acceptance ran, *not* a signature by the incoming operator. See §5.1. |
| Bill-of-materials walk | your application | Not in this library, because a walk fetches. A walk checks each node's pin, caps the depth and the node count, detects cycles by path, and fails closed. |

The primitives are sound. `PassportRef`'s hash pin is the right idea, and bounding the
walk and handling cycles belong to your application. The gaps below are about *what the edges
mean*, not how they are fetched or checked.

---

## 2. Regulatory requirements

The conventions of `regulatory/COMPLIANCE.md` apply: a claim that cannot be pinned to the
Official Journal text is marked, not asserted.

### 2.1 Second life: Battery Regulation (EU) 2023/1542, Art. 77(7)

The operative sentence, as published:

> For a battery that has been subject to preparation for re-use, preparation for
> repurposing, repurposing or remanufacturing, the responsibility for the
> fulfilment of the obligations under paragraph 4 of this Article shall be
> transferred to the economic operator that has placed that battery on the market
> or has put it into service. Such battery shall have a new battery passport
> linked to the battery passport or passports of the original battery or
> batteries.

Four requirements follow directly from that text:

- **R1: a second-life unit gets a *new* passport.** Not an edit to the old one.
- **R2: the new passport links to the passport or passports of the original battery or
  batteries, plural on both sides.** One second-life unit may derive from *several*
  predecessors. This is not a corner case: a stationary storage pack assembled from
  several retired EV packs is the typical second-life product.
- **R3: responsibility transfers** to the operator placing the second-life unit on the
  market or putting it into service. The link and the responsibility move are one event,
  not two.
- **R4: four distinct operations** are named: *preparation for re-use*, *preparation for
  repurposing*, *repurposing* and *remanufacturing*.

**COMPLIANCE-PIN: EU 2023/1542, Art. 77(7) (OJ L 191, 28.7.2023, p. 73).** Read directly
from the Official Journal text. The quoted sentence is the first subparagraph of
**paragraph 7**, and the "obligations under paragraph 4" it transfers are Art. 77(4)'s
duty to keep passport information "accurate, complete and up to date". The earlier open
point, the paragraph number within Art. 77, is closed.

The **second subparagraph of the same paragraph** covers the waste transition, and it
answers §6 question 4 directly:

> Where the status of a battery changes to that of a waste battery, the
> responsibility for the fulfilment of the obligations under paragraph 4 of this
> Article shall be transferred either to the producer or, where appointed in
> accordance with Article 57(1), the producer responsibility organisation, or the
> waste management operator selected in accordance with Article 57(8).

- **R5: the waste transition moves responsibility again**, to one of three named
  recipients, and, unlike the first subparagraph, it requires **no new passport**: the
  battery keeps its passport, and the status change is recorded as a new *version* of it.
  Responsibility moves while the passport stays. See §4.2.

**Art. 77(8)** gives the only termination the article states: "A battery passport shall
cease to exist after the battery has been recycled." Recycling, not consumption by a
second-life unit, is what ends a passport. This answers §6 question 3.

The four operations are defined terms, and the boundary between the two repurposing ones
is the **waste status of the input**, not the actor:

- **Art. 3(30)**, *preparation for repurposing*: "any operation, by which a **waste
  battery**, or parts thereof, is prepared so that it can be used for a different purpose
  or application than that for which it was originally designed".
- **Art. 3(31)**, *repurposing*: "any operation that results in a battery, **that is not a
  waste battery**, or parts thereof being used for a purpose or application other than
  that for which the battery was originally designed".

**COMPLIANCE-PIN: EU 2023/1542, Art. 3(29) to (32) (OJ L 191, 28.7.2023, p. 27).**

### 2.2 Change of status: Annex XIII point 4(c)

A battery that has undergone one of the four operations must carry information on its
**change of status**, reported under **point 4 of Annex XIII** and reachable through the
QR code. The enumerated values, as published:

> (c) information on the status of the battery, defined as 'original',
> 'repurposed', 're-used', 'remanufactured' or 'waste';

**COMPLIANCE-PIN: EU 2023/1542, Annex XIII point 4(c) (OJ L 191, 28.7.2023, p. 109).** Read
directly from the Official Journal text.

**The value list this document previously carried was wrong in two of five places**, which
is what the pin exists to catch. It read *"original use, re-used, repurposed,
remanufactured, approaching end of life"*. The Official Journal says `'original'`, not
"original use", and **"approaching end of life" is not a status value in this Regulation
at all**: the string occurs nowhere in its text, and the fifth value is `'waste'`. Encoding
the unpinned list would have shipped a status value that does not exist in EU law.

Two further facts follow from where point 4 sits, and both bind Phase 5:

- **Point 4 is the legitimate-interest tier.** Its heading is "INFORMATION AND DATA
  RELATING TO AN INDIVIDUAL BATTERY ACCESSIBLE ONLY TO PERSONS WITH A LEGITIMATE
  INTEREST". `life_status` is therefore **not public**. `Disclosure::Individual` already
  has exactly this meaning (its own doc names "status"), so the field is classified there
  rather than defaulting to public.
- **Status is reported on change, over the life of one record.** Point 4(a) requires the
  performance and durability values "when the battery is placed on the market **and when
  it is subject to changes in its status**". A status reported on change is not a
  create-time constant. See §4.

### 2.3 ESPR (EU) 2024/1781

ESPR is the framework and has no bill-of-materials or second-life linkage article of its
own. What it does have, and what therefore binds every edge here:

- **Art. 9(1):** passport data "shall be accurate, complete and up to date".
- **Art. 11(1)(e):** the passport remains available after insolvency, liquidation or
  cessation of activity of the responsible operator.

The consequence for lineage: an edge that points at a passport whose operator has since
disappeared must still resolve. Per-product-group delegated acts are where
bill-of-materials granularity will be specified, and none is in force for our product
groups yet, so **the library must not hard-code any product group's notion of
"component".**

---

## 3. Gap analysis

### G1: the model could not express the plural case (violated R2). Closed

`parent_passport_ref: Option<PassportRef>` held **one** predecessor. The regulation says
"passport **or passports** of the original battery **or batteries**". A storage pack built
from four retired EV packs could not be represented. That was a data-model defect against
the plain text.

Closed in Phase 2: `derived_from: Vec<DerivationRef>`.

### G2: two mechanisms modelled one regulatory event (violated R3). Closed

`TransferReason` has carried the full Art. 77(7) vocabulary (all four operations) since
Phase 1, but it lives on `TransferRecord`, which hands responsibility over on a *single,
continuing* passport. `derived_from` creates a *new* passport and names the operation. The
two vocabularies agreed, and nothing required them to.

R1 and R3 are the same event: a new passport **and** a responsibility move. An operator
could perform either half alone and nothing detected the inconsistency. Two mechanisms for
one event is how they drift.

Closed in Phase 4: `dpp_rules::lineage::substantiation` requires them to agree, so the two
mechanisms describe one event or the passport is reported as unsubstantiated.

### G3: `TransferReason` was missing one of the four operations (violated R4). Closed

Art. 77(7) names four operations. `TransferReason` had three: `PreparationForReuse`,
`Repurposing`, `Remanufacturing`. **`preparation for repurposing` was missing**, so a
transfer made for that reason could only be recorded as one of the other three.

An earlier draft said the distinction was "a distinct actor in the text". That is not what
the text says, and the correction matters for anyone implementing the boundary:
**Art. 3(30) and Art. 3(31) separate the two by the waste status of the input**.
Preparation for repurposing works on a *waste* battery, repurposing on one that is *not* a
waste battery, and the same actor may perform both.

Closed ahead of Phase 2: adding a variant to `TransferReason` is additive, and the enum is
`#[non_exhaustive]`, so it needed neither Phase 2's `SecondLifeOperation` nor a breaking
release. It does **not** settle the separate open question in
`docs/regulatory/COMPLIANCE.md` about whether the second-life operations belong on
`TransferReason` at all. It makes the set consistent with the article it comes from, so
that if those variants are later withdrawn, all four go together.

### G4: the edge was untyped. Closed

`PassportRef` records *where* and *which hash*, never *what relation*. Direction was encoded
in the field name, a deliberate simplification that stopped being enough:

- Upward, the four Art. 77(7) operations have different legal consequences, so the edge
  must say which one occurred. **Closed in Phase 2:** `DerivationRef` carries a required
  `SecondLifeOperation`.
- Downward, a bill-of-materials edge with no quantity or role cannot answer "how much of
  what, where", which is the question a bill of materials exists to answer. **Closed in
  Phase 3:** `ComponentRef` carries an optional `quantity` and `role`, which the library
  transports and never interprets (G8).

### G5: no product-life status axis. Closed

`PassportStatus` (`Draft`, `Published`, `Suspended`, `Retired`, `Superseded`,
`Deactivated`) is a **publication** lifecycle. Annex XIII point 4(c) wants a
**product-life** status (`original`, `repurposed`, `re-used`, `remanufactured`, `waste`),
pinned in §2.2. The two are independent: a repurposed unit's passport is `Published`.
`Deactivated` matches Art. 77(8)'s post-recycling end state and stays as it is.

Closed in Phase 5: `LifeStatus` carries the second axis, with the Official Journal
spellings as its wire forms. Note `re-used`, hyphenated, which is the annex's own spelling
and not a typo to tidy. It is `Option` because only Reg. (EU) 2023/1542 defines this
vocabulary: a textile passport asserting `original` would borrow a battery term for a
question its own instrument does not ask. §4.2 records why this field follows a different
rule from the lineage edges above it.

### G6: neither field was protected from `patch_fields`. Closed

The original issue. Neither `parentPassportRef` nor `componentRefs` was in
`PROTECTED_PATCH_FIELDS`, so both could be written through a free-form field patch, the
same bypass already closed for `operatorIdentifier` and `facility`. Both are in the list
now (Phase 0). §4 covers what that protection may and may not be read to mean.

### G7: a lineage edge was asserted, never substantiated. Closed for second life

The hash pin proves the *target* has not been modified. It does not prove that the
operation the edge names took place. Anyone can publish a passport claiming to derive
from, or contain, anyone else's product.

For a bill of materials this is mostly harmless (over-claiming a supplier is a commercial
problem), and it stays that way on purpose; see §5.1. For second life it is not: R3 moves
regulatory responsibility, and a false claim purports to end the original operator's
Art. 77(4) duty for a unit that may still be on the market under their name.

Closed in Phase 4 for the derivation direction: `dpp_rules::lineage::substantiation`
refuses a second-life edge that no transfer of responsibility supports. The corroboration
it relies on is the **outgoing** operator's authorisation, the predecessor's own, for the
reason §5.1 gives (which is not the one this document originally gave). The rule was first
named `consent`; Art. 77(7) moves the responsibility by operation of law and asks for
nobody's agreement, so the rule asks for evidence, not consent.

### G8: "component" is undefined across product groups

Left open on purpose, and it should stay open: a battery module, a fibre lot and an
electronics sub-assembly are different kinds of thing, and no delegated act defines
granularity for our in-force product groups yet. The library's job is to carry a pinned
reference plus product-group-neutral qualifiers, and let product group plugins interpret
them.

---

## 4. The central question: when may a lineage edge change?

The issue asked whether these fields are set once at creation or attachable over time. The
answer follows from a rule the code already enforces elsewhere.

**A field in the signed public view that changes after publish makes the served body stop
verifying against its own signature.** `AccessPolicy::passport_default()` states this
invariant in code, and it is why `lintResult` was moved off the public tier.

Both lineage fields are in the signed public view, so:

- **`derived_from` (upward) is fixed once signed.** Per R1, the second-life passport *is*
  the new record, and its predecessors are known when it is created.
- **`component_refs` (downward) is fixed once signed, and a bill-of-materials change on a
  published passport is a new passport version.** The mechanism already exists:
  `supersedes_id` and `version`. Changing a published bill of materials in place would
  bring back exactly the defect just fixed for `lintResult`.

**No method for attaching edges over time is needed**, and both fields are in
`PROTECTED_PATCH_FIELDS`. If a sub-assembly's passport arrives after publication, the
right response is to supersede with a new version that includes it. That keeps every
signature valid and leaves an auditable trail, instead of silently rewriting a signed body.

### 4.1 The invariant is *immutable once signed*, not *set at creation*

An earlier draft derived the rule from the signed public view and then generalised it to
"set at creation by construction". Those are different claims, and the stronger one is
wrong.

**The draft window is legitimate.** Between creation and first publish there is no
signature to break, and assembling a bill of materials step by step before first publish
is the normal case. The only shipping backend already depends on this: it derives its
protected list from `PROTECTED_PATCH_FIELDS` with `componentRefs` removed, because the
update path it serves accepts drafts only. This section as previously written forbade
what that backend does; the backend was right and the rule was overstated.

**So the exception belongs in the rule, with its condition attached.** An implementation
should be able to say *"editable while `Draft`"* rather than *"not protected"*. Stated as
an absence from a list, the exception is unconditional at the trait boundary; stated as a
condition, it survives a caller that did not expect it.

**The distinction matters because the guard is not where the exception is.**
`patch_fields` checks passport status on neither side; the draft-only check lives in the
*service that calls it*. A second caller already reaches `patch_fields` with **published**
rows: the lint re-check, which stores `lintResult` on every published passport. Nothing is
broken today, because that caller patches one field and does not touch lineage. But what
keeps a third caller safe is this crate's list, and `componentRefs` is exactly the entry a
backend removes from it. The invariant then rests on every current and future caller
re-checking status, the same pattern of drift that put `operatorIdentifier`, `facility`
and `parentPassportRef` at risk before.

Any second implementation faces the same choice with no guidance and no reason to make it
the same way. This section had to fix that before Phase 2 rewrote these fields.

### 4.2 `life_status` is not governed by this rule

Phase 5's field is a different case, and the rule above must not be extended to it by
analogy.

Per §2.2 it is `Disclosure::Individual`, so it is not in the signed *public* view, and per
the second subparagraph of Art. 77(7) the **waste transition requires no new passport**. A
battery whose status changes to waste moves responsibility while keeping its passport, and
Annex XIII point 4(a) expects values reported "when it is subject to changes in its
status". A `life_status` fixed at creation could never reach `'waste'`, one of the five
values the law lists.

The four Art. 77(7) operations *do* each produce a new passport (R1), so those four values
are set at creation. `'waste'` is the transition that is not, and it is why `life_status`
needs a defined change path. It is not a free patch field either, since the transition is
also a responsibility move under R5.

**Phase 5 specified that path: the waste transition is a new passport version.** An earlier
draft expected the path to be an alternative *to* `PROTECTED_PATCH_FIELDS`. It is the
opposite: the path is `supersedes_id` plus `version`, which is §4's existing answer to "a
signed field must change", and being in `PROTECTED_PATCH_FIELDS` is what forces a caller
onto it. `life_status` is therefore in that list with the other signed fields.

**A version is not a new passport in the Art. 77(7) sense.** The two terms describe
different things. A new passport (R1) is issued for a different unit and links to its
predecessor through a derivation edge. A new version is the same passport for the same
unit: a successor record whose `supersedes_id` points at the version before it, with the
next `version` number, and no derivation edge. The waste transition is a version, so the
battery's passport continues, as §2.1 and §6 say.

The alternative was a dedicated port method. It does not fit the rest of the design: it
would break the served body's signature and have to re-sign, which is a version bump in
disguise, and it would add a second change mechanism for the one field that least needs
one. Versioning also leaves the audit trail a responsibility move under R5 should leave.

`PassportStatus::Superseded` is the right publication status for the old version here: it
really is a new version of the same product. That differs from a predecessor *consumed* by
a second-life unit, which stays `Published`, since Art. 77(8) makes recycling the only
termination.

---

## 5. The model

**Every type below shipped:** `DerivationRef` and `SecondLifeOperation` in Phase 2,
`ComponentRef` and `Quantity` in Phase 3, all in `dpp-domain::passport`, and `LifeStatus` in
Phase 5, with `TransferReason::WasteHandover` and
`dpp-rules::lineage::check_life_status_consistency`.

`PassportRef` stays exactly as it is: a plain "where plus pin" primitive, correct and
direction-neutral. Each direction wraps it with the qualifiers it needs.

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
/// enumerated list, pinned in §2.2: not a superset, and not renamed.
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
    /// Product-group-neutral quantity ("2", "1.4 kg"). Interpreted by product
    /// group plugins, never by the library.
    pub quantity: Option<Quantity>,
    /// Product-group-defined role of this constituent ("cell", "outer shell").
    pub role: Option<String>,
}

/// `unit: None` means a dimensionless count: two of a thing, not two kilograms
/// of it. `f64` matches every other physical quantity in the crate
/// (`MaterialEntry::weight_kg`); the library does no arithmetic on it. The unit
/// is an opaque label, not a vocabulary the library validates, which would be the
/// library deciding a product group's semantics (G8 forbids that).
pub struct Quantity {
    pub value: f64,
    pub unit: Option<String>,
}
```

On `Passport`:

```rust
// was: parent_passport_ref: Option<PassportRef>
pub derived_from: Vec<DerivationRef>,   // closes G1, G4 upward
// was: component_refs: Vec<PassportRef>
pub component_refs: Vec<ComponentRef>,  // closes G4 downward
pub life_status: Option<LifeStatus>,    // closes G5
```

`life_status` is `Option` because Annex XIII point 4 is Battery Regulation content and no
other in-force product group requires it. `None` means "this product group reports no
product-life status", which is different from `Some(Original)`.

### 5.1 Binding lineage to responsibility (closes G2 and G7)

**A second-life operation already *is* a transfer of responsibility under Art. 77(7)**, and
a transfer record already carries the outgoing operator's own authorisation. That is the
corroborating artefact G7 needs and the responsibility move R3 requires, in one artefact
that already exists.

**Correction: an earlier version of this section justified this differently, and was
wrong.** It said a `TransferRecord` is "dual-signed by both the outgoing and incoming
operator". It is not, and `TransferRecord` says so at the field: the acceptance half is the
**hosting node's attestation that the acceptance step ran**, not a signature by the incoming
operator, who has no key on the node that records it. The field was renamed from
`to_signature` precisely because the old name invited that misreading, and this document
then made it anyway.

The conclusion holds, for a reason that is easy to get backwards. **The corroboration this
rule asks for is the predecessor's**, and in a second-life transfer the predecessor's
operator is the *outgoing* party, so the half that is a real party signature is exactly the
half that matters. The incoming half would have been this passport's own operator
asserting something about itself, which was never evidence. Acceptance is still checked,
because an abandoned or rejected handover should not support a live claim, but it is
checked as a *status*, not as a second signature.

So the two are bound rather than adding a third mechanism:

> A passport carrying a non-empty `derived_from` must reference a `TransferRecord` for each
> predecessor, whose `TransferReason` matches that edge's `SecondLifeOperation`, and whose
> incoming operator is this passport's operator.

That one rule closes two gaps: the two mechanisms become one event (G2), and a second-life
claim carries the predecessor operator's authorisation (G7). It shipped in Phase 4 as
`dpp_rules::lineage::consent`, since renamed `substantiation`: a pure cross-field rule,
which is where it belongs.

This is the model's rule, not the Regulation's. Art. 77(7) moves the responsibility by
operation of law and requires no transfer record to exist, so a missing record makes a claim
**unsubstantiated**, not a breach. Whether the rule should keep requiring the outgoing
operator's authorisation, which a lawful second-life operator may be unable to obtain, is
tracked in #499.

The rule also depended on G3 being closed first: it matches a `SecondLifeOperation` against
a `TransferReason`, so a `TransferReason` unable to express `preparation for repurposing`
would have made the rule unsatisfiable for one of the four operations.

**What the rule cannot do.** It cannot tell whether a transfer record concerns the
predecessor a given edge points at: an edge identifies its target by URI, a transfer
identifies its subject by passport id, and resolving one to the other is a network fetch.
So **the caller correlates and the rule checks**. An edge for which the caller found no
transfer is reported as unsubstantiated, the correct reading of "no evidence was found",
rather than an assumption that none exists. The split follows from what the library cannot
do: your application resolves the URI and correlates the transfer, and
`dpp_rules::lineage::substantiation` judges the evidence it found.

Bill-of-materials edges get **no** substantiation requirement: it would demand a signature
from every supplier for every assembly, which no supply chain will produce. A
`componentRef` is a *claim by the assembler*, pinned so it cannot be tampered with, and a
walk that checks the pin already reports exactly that.

### 5.2 What stays out of the library

- Any product group's definition of "component", or its granularity (G8).
- Fetching, resolving and walking edges: your application's job.
- Any status value beyond the five Annex XIII point 4(c) lists. The list is pinned (§2.2)
  and closed: a sixth value would be an invention, which is what the unpinned draft of this
  document produced.

---

## 6. Questions resolved

All four are settled. Two were answerable from the Official Journal text and are pinned; two
were design decisions and are recorded as decided.

1. **Is `life_status` core or product group data? Core, optional, and not
   compliance-gating.** It comes from the Battery Regulation, but ESPR delegated acts may
   generalise it, and the shape (a small closed enum on the passport envelope) is
   product-group-neutral even though the obligation is not. Modelled as
   `Option<LifeStatus>`; no rule gates on it. Its disclosure class is **not** a design
   decision: Annex XIII point 4 fixes it at `Disclosure::Individual` (§2.2).

2. **Does a second-life passport inherit its predecessors' bill of materials? No.**
   Art. 77(7) requires the new passport to be *linked* to the predecessors' passports, not to
   re-declare their contents. Copying would duplicate data that can go stale, and the
   predecessors keep their own passports (question 3), so the data stays reachable through
   the edge.

3. **What happens to the predecessor's passport? It stays `Published`.** The text decides
   it: **Art. 77(8)** gives the only termination, "A battery passport shall cease to exist
   after the battery has been recycled." Being consumed by a second-life unit is not
   recycling, so nothing terminates it. `Superseded` remains wrong for the reason already
   given (it means a new *version* of the same product), and no new `PassportStatus` value is
   needed. The derivation edge records what happened, which is what Art. 77(7) asks for.

4. **The waste transition. Resolved by the second subparagraph of Art. 77(7)** (quoted in
   §2.1), which had not been read when the question was written. Responsibility moves to one
   of three named recipients: the producer, the producer responsibility organisation where
   appointed under Art. 57(1), or the waste management operator selected under Art. 57(8).
   Two consequences:
   - It requires **no new passport**, so the passport continues and its `life_status`
     becomes `'waste'` in a new version of it. §4.2 explains how that change is made.
   - It is a genuine transfer of responsibility under that subparagraph, so it needed a
     `TransferReason` of its own. Phase 5 added `TransferReason::WasteHandover`, because the
     recipients are the waste-operator vocabulary rather than the second-life one.
     `PassportStatus::Deactivated` still covers only the Art. 77(8) post-recycling end state,
     a later and distinct event.

---

## 7. Phased plan

| Phase | Scope | Breaking | State |
|---|---|---|---|
| **0** | Add `parentPassportRef` and `componentRefs` to `PROTECTED_PATCH_FIELDS` (G6). One line; closes the live bypass and rules nothing out. | no | **landed** |
| **1** | Pin Art. 77(7), Art. 3(29) to (32) and the Annex XIII point 4(c) status list against the Official Journal; reconcile §4; resolve the §6 questions; add the `TransferReason` variant (G3). | no | **landed** |
| **2** | `DerivationRef`, `SecondLifeOperation` and plural `derived_from` (G1, G4 upward). | **yes** | **landed** |
| **3** | `ComponentRef` with quantity and role (G4 downward). A bill-of-materials walk reads the new shape. | **yes** | **landed** |
| **4** | The rule binding lineage to transfers, in `dpp-rules` (G2, G7). The caller correlates an edge to a transfer, since that needs a URI resolved; the rule judges the result. | no | **landed** |
| **5** | `life_status` (G5) with the §2.2 value list and `Disclosure::Individual`; its change path per §4.2; `TransferReason::WasteHandover` (§6 question 4). | no | **landed** |

Phase 0 could land on its own and did not wait for the rest. Phase 1 also carried G3: the
variant is additive on a `#[non_exhaustive]` enum, so it needed neither Phase 2's breaking
release nor `SecondLifeOperation`, and Phase 4's rule is unsatisfiable without it.

Phases 2 and 3 are independent of each other; 4 needs 2's vocabulary; 5 depended only on
Phase 1's pin.

---

## 8. Compatibility

Phases 2 and 3 changed published field names and types (`parentPassportRef` to
`derivedFrom`; `componentRefs` elements became objects). Both are done, and under the
lockstep versioning policy they form one coordinated minor bump across all core crates.
**They had to ship in the same release**, which is why they were sequenced together.

**Correction: an earlier version of this section named a mechanism that does not exist.** It
said the read-time upcast lens (`?schema_view`) "is the existing seam" and "should carry the
migration". It cannot. `Passport::from_stored` lenses `productGroupData` and nothing else,
and both it and `Passport::schema_version` say so: **envelope fields are never lensed, on
purpose**, because a lens transforms one product group's sub-object while an envelope field
is shared by every product group's documents, so a faulty envelope transform would corrupt
all of them at once.

The envelope's actual rule is additive only: `Option<T>` with `#[serde(default)]`, or a
rename that keeps accepting the old key. **Phase 2 did not follow it**, knowingly: there are
no published passports to strand, so the cost was zero. Two things follow, and neither
should be inherited without a fresh decision:

- **The failure was silent, and is now loud.** `Passport` sets no `deny_unknown_fields` and
  `derived_from` has a default, so a document still carrying `parentPassportRef`
  deserialised *successfully* with no lineage edge. That is quieter, and worse, than the
  `sector` to `productGroup` rename, where the renamed field was required and older
  documents refused to load. `REMOVED_ENVELOPE_KEYS` now names the old key, and
  `Passport::from_stored` refuses any document carrying one. A change that removes a key
  adds its own entry there; changing an element *type* fails loudly on its own, because
  `#[serde(default)]` applies only when the key is absent.
- **This stops being free when the first real passport is published.** After that, an
  envelope rename must keep the old key or ship a one-time document rewrite in the publish
  pipeline. A signature covers the old key names, so a rewritten document does not verify,
  and a silently emptied one verifies while being wrong.

Phase 3 turned `componentRefs` elements into objects, which is a different failure again.
The key name did not change, and `#[serde(default)]` applies only when a key is *absent*, so
a document carrying the old element shape would fail to deserialise on its own, loudly,
needing no `REMOVED_ENVELOPE_KEYS` entry.

**Loud is the wrong answer here, and that is the more important lesson.** The first analysis
asked only where a document is *stored*. It is also **fetched**: `componentRefs` is in the
signed public view, and other operators' nodes read it over the network to verify a bill of
materials. A fetched passport is signed and belongs to someone else; nobody can rewrite it
into the new shape. Refusing it refuses data that is correct, current and unforgeable,
permanently.

The refusal does not degrade gracefully either. A verification walk that cannot parse an
entry reports a malformed reference, which is graded as an **integrity violation**, in the
same class as a hash mismatch or a cycle. An upgraded node would therefore report an older
node's bill of materials as *tampered*, on evidence that is only a version difference. Nodes
are independent per-operator deployments, so that skew is the steady state, not a window.

`ComponentRef` therefore implements `Deserialize` by hand and accepts both shapes. That is
the same *"a rename that keeps accepting the old key"* rule the envelope already follows,
applied to an element type instead of a key name.

**The distinction to carry forward:**

| Change | Stored document | Fetched document |
|---|---|---|
| Rename a key | silent: needs a check | silent loss of lineage |
| Change a present key's type | loud: reports itself | **loud, and reads as tampering** |

A field in the signed public view answers to the second column. Deciding its compatibility
from the first alone nearly shipped this wrong.

**Phase 2 is not fully covered by this.** The `derivedFrom` rename degrades silently across
nodes rather than accusing anyone, which is milder, but a GS1 resolver that reads
`parentPassportRef` from fetched JSON to build predecessor link relations emits none on a
version-skewed node. That is a deliberate open decision.
