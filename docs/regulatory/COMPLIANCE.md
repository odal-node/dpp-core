# Regulatory Compliance Transparency

This document maps dpp-core's functionality to the EU regulations it
implements, tracks the standards it aligns with, and describes how regulatory
changes are incorporated.

## Regulatory Scope

dpp-core provides the domain types and validation logic for Digital Product
Passports mandated by:

| Regulation | Reference | Status | dpp-core Coverage |
|---|---|---|---|
| ESPR | Regulation (EU) 2024/1781 | In force | Core passport model, per-audience field disclosure, schema validation |
| Battery Regulation | Regulation (EU) 2023/1542 | In force, DPP deadline Feb 2027 | Battery schemas (v1.0.0, v2.0.0 … v2.6.0), product group plugin, per-category content rules |
| Textile (anticipated) | ESPR delegated act (draft) | See timeline note below | Textile schemas (v1.0.0, v1.1.0), product group plugin |
| CBAM | Regulation (EU) 2023/956 | In force | Steel schema (v1.0.0), embedded emissions fields |

> **Timeline note:** the textile DPP delegated act has no adopted date yet; published estimates range from ~2026 to 2028. Treat any specific year here as provisional and verify against EUR-Lex before relying on it.

dpp-core does **not** implement infrastructure concerns (HTTP APIs, databases,
registry connectivity). Those belong in the platform layer.

## Standards Alignment

| Standard | Body | Version Tracked | Where Used |
|---|---|---|---|
| GS1 Digital Link | GS1 | v1.2 | `dpp-digital-link` — URL parsing and link-type negotiation |
| AAS (Asset Administration Shell) | IDTA | v3.0 | `dpp-aas` — submodel mapping |
| W3C Verifiable Credentials | W3C | Data Model v2.0 | `dpp-crypto` — VC issuance and verification |
| DID:web | W3C | did:web Method Spec | `dpp-crypto` — DID document builder |
| JSON Schema | IETF | Draft 2020-12 | `dpp-domain` — passport data validation |
| CEN/CENELEC JTC 24 | CEN | Six EN 182xx:2026 harmonised; two still draft | Conformance target, not a term source — no clause text read |

> **On the JTC 24 row.** Whether these standards are cited in the OJEU — and so
> whether conformity to one carries a presumption of conformity — is a fact with
> one home: the `jtc24` record in `dpp-vocab`'s `vocabularies/`, which carries the
> source and the date it was checked. This table names the standard; it does not
> restate that record's finding. An earlier version of this row said "Draft
> (monitoring)" and was stale, which is the reason for the split.

## Schema-to-Regulation Mapping

Each JSON schema under `schemas/` is traceable to its regulatory basis:

| Schema File | Regulation | Key Articles/Annexes | Notes |
|---|---|---|---|
| `battery/v1.0.0.json` | 2023/1542 | Art. 77, Annex XIII | Minimum viable battery passport |
| `battery/v2.0.0.json` | 2023/1542 | Art. 77, Annex XIII | Extended fields for carbon footprint |
| `battery/v2.6.0.json` | 2023/1542 | Annex VI Part A, Annex XIII points 1–4 | Current. All four disclosure tiers, per-property `x-disclosure` |
| `textile/v1.0.0.json` | ESPR 2024/1781 | Art. 9-10 (framework) | Baseline textile passport |
| `textile/v1.1.0.json` | ESPR 2024/1781 | Art. 9-10 + anticipated delegated act | Adds fibre composition, durability, microplastics |
| `unsold-goods/v1.0.0.json` | ESPR 2024/1781 | Art. 25 / Annex VII | Unsold-goods destruction-ban compliance |
| `steel/v1.0.0.json` | CBAM 2023/956 | Art. 7 | Embedded emissions, production origin |
| `electronics/v1.0.0.json` | ESPR 2024/1781 | (delegated act anticipated) | Repairability, spare parts, substances |
| `aluminium/v1.0.0.json` | ESPR 2024/1781 / CBAM | (delegated act anticipated) | Production route, CO₂e/tonne, recycled content |
| `construction/v1.0.0.json` | CPR 2024/3110 | (delegated acts anticipated) | Construction product fields |
| `detergent/v1.0.0.json` | ESPR 2024/1781 | (delegated act anticipated) | Surfactant / ingredient fields |
| `furniture/v1.0.0.json` | ESPR 2024/1781 | (delegated act anticipated) | Furniture product group fields |
| `toy/v1.0.0.json` | EU 2025/2509 (Toy Safety) | (delegated act anticipated) | Toy product group fields |
| `tyre/v1.0.0.json` | ESPR 2024/1781 | (delegated act anticipated) | Tyre product group fields |

The current schema version per product group is resolved by `ProductGroupCatalog`, not hardcoded at call sites. See `docs/regulatory/REGULATORY.md` for which product groups carry implemented compliance rules vs. placeholders.

## Regulatory Change Process

When an EU regulation or delegated act is published or amended:

1. **Monitor** — Track publications via EUR-Lex and JTC 24 mailing lists.
   This is recorded in the project roadmap.
2. **Assess** — Open a GitHub issue tagged `regulatory` describing the change
   and its impact on dpp-core's schemas, domain types, or compliance rules.
3. **Implement** — Create a new schema version (never modify existing ones).
   Update domain types if required. All changes go through the standard PR
   process.
4. **Document** — Update this file's mapping tables. Add a CHANGELOG entry
   referencing the regulation.
5. **Release** — Cut a new version per the [release process](../governance/RELEASE.md).

### Schema Immutability Rule

Published schema versions are **never modified**. If a regulation changes
requirements, a new version is created. This ensures that passports validated
against v1 remain valid against v1 indefinitely, even after v2 is published.

## Compliance Architecture

dpp-core exposes a pluggable determination seam for product-group-specific compliance:

```
ComplianceRegistry (port trait)
  ├── PassthroughRegistry (dpp-core, Apache) → PassthroughNoValidation, computes nothing
  └── plugin-backed registry (platform)      → Wasm product group plugins (product-group-battery, etc.)
```

The `ComplianceRegistry` trait defines `compute(&self, product_group: Product group, data:
&ProductGroupData) -> Result<ComplianceResult, ComplianceError>`; the per-product group
`ComplianceStrategy` trait defines `compute(&self, data: &ProductGroupData) ->
Result<ComplianceResult, ComplianceError>`. The Apache default
(`PassthroughRegistry`) computes nothing and returns
`PassthroughNoValidation` for every product group; real determinations come from the
Wasm product group plugins (or a proprietary `PremiumComplianceRegistry`). A computed
determination is passed through `gate_determination(catalog.is_in_force(product group),
…)` so a provisional product group can never surface a binding result. This separation
means:

- **Core stays generic** — no product-group-specific determination logic in the workspace crates.
- **Regulations are isolated** — a Battery Regulation change only touches
  `product-group-battery` (and the shared rules in `dpp-rules`).
- **New product groups are additive** — adding a new delegated act means adding a
  schema file, a catalog manifest, and a plugin.

## Cryptographic Compliance

| Primitive | Algorithm | Library | Purpose | Regulatory Basis |
|---|---|---|---|---|
| Signing | Ed25519 | `ed25519-dalek` | Passport authenticity, JWS | ESPR Art. 11(g) (data authentication, reliability and integrity) |
| Encryption | AES-256-GCM | `aes-gcm` | Restricted-field protection at rest | Reg. (EU) 2023/1542 Art. 77(2) (audience/disclosure) |
| Hashing | SHA-256 | `sha2` | Data fingerprinting | General integrity |
| Entropy | OS CSPRNG | `rand` | Key generation | Cryptographic best practice |

These choices are documented in [SECURITY.md](../../SECURITY.md) and
the architecture docs under `docs/architecture/IDENTITY.md`.

## EU Registry Readiness

The EU DPP Registry (ESPR Article 13) **became operational on 20 July 2026**,
together with a testing environment and User Guidelines. Its operating rules are
Commission Implementing Regulation (EU) 2026/1778.

Verified against the OJ text of IR 2026/1778 (adopted 16 July 2026, published
17 July 2026, in force on the twentieth day thereafter — **Art. 24**):

- **Who may register.** A digital product passport is registered by a *verified
  economic operator* placing the product on the market (**Art. 8(1)**).
  Verification is by eIDAS credential: for a legal person, a **qualified
  electronic seal** supported by a qualified certificate issued by a QTSP, or a
  qualified electronic attestation of attributes (**Art. 4(2)**). Establishment
  in the Union is **not** a precondition — Art. 4(2)(b) provides expressly for
  operators not required to be so established.
- **Registration by a third party is permitted.** Where a verified economic
  operator authorises a third party to perform registration actions on its
  behalf, that third party must itself complete the verification process of
  **Art. 5** (value chain actors — which the recitals enumerate as including a
  digital product passport service provider). The economic operator "shall
  remain fully responsible for compliance with the obligations set out in this
  Regulation" (**Art. 19(4)**) and is the controller of the data it submits
  (**Art. 19(5)**). Delegating the mechanics does not move the liability.
- **Registry structure** (**Art. 3**) includes an API for registering passports
  and retrieving information (Art. 3(b)), a list of verified digital product
  passport service providers (Art. 3(f)), and a storage component for unique
  identifiers *and commodity codes for products intended to be placed under the
  customs procedure "release for free circulation"* (Art. 3(e)).
- **Granularity.** Registration occurs at model, batch or item level as the
  applicable delegated act requires (**Art. 8(1)**); where rules conflict, at
  the most granular level required (**Art. 8(3)**). An item-level passport must
  link **both** batch and model identifiers where those exist (**Art. 8(4)**);
  a batch-level passport must link the model identifier (**Art. 8(5)**).
- **Automated checks on submission** (**Art. 8(7)**): semantic conformity,
  coherence of mandatory data, conformity with the required granularity level,
  and — *where relevant* — validity of the commodity code.
- **Transfer of registered passports** (**Art. 6a**): a registered DPP "may be
  transferred to another verified economic operator or, where applicable, to a
  verified value chain actor that takes over the obligations from the previous
  actor ... from the date indicated for the transfer." Recital (10) gives the
  triggers as failed identity verification and organisational change (merger,
  split, sale of the actor, cessation). **The registry is therefore the
  authoritative record of who holds the obligations, and Art. 5(3) closes it to
  everyone but verified actors.** Full treatment, including how this bounds what
  `domain::transfer` may claim, is under *Transfer-of-Responsibility Article
  Pin* below.

dpp-core's `dpp-registry` crate is a **ghost connector** carrying preparatory
interface types that **predate the published specification**:

- `RegistrationPayload`, `EuRegistryEnvelope`, `EuRegistryResponse`,
  `StatusResponse`, `TransferNotification`, the four Art. 13 identifier structs
  (`ProductIdentifier`, `ProductItemIdentifier`, `FacilityIdentifier`,
  `OperatorIdentifier`), error types, and `RegistryEndpoint` — anticipated data
  shapes based on published ESPR articles and JTC 24 draft discussions.
- `RegistrySyncPort` — the port trait (defined in `dpp-domain::ports`, with a
  `GhostRegistrySync` placeholder) that the platform implements once the
  official API specification is released.

These types remain explicitly unstable. What the OJ text *fixes* has now been
reconciled against it; what only the API specification can fix has not, and is
listed separately below so the two are never confused.

### Reconciled against the OJ text (0.16.0)

- **Registration granularity** — `RegistrationLevel` carries the model / batch /
  item level (Art. 8(1)) and the identifiers Art. 8(4) and 8(5) require it to
  link. Absence of a model or batch identifier is treated as lawful, because
  both obligations are conditional on such a design existing and recital (14)
  is explicit that products unique by nature have neither. `item_id` is now
  conditional on the declared level rather than unconditional.
- **Commodity code** — represented by `CommodityCode` on the passport and on the
  registration payload, validated *structurally* as HS-6, CN-8 or TARIC-10.
  Whether a code falls inside the range a product group permits (Art. 8(7)(d))
  is **not** checked here: those ranges live in the applicable delegated act.
  Absence remains lawful — the obligation is qualified "where relevant".
- **Operator identifier scheme** — carried explicitly rather than assumed, so a
  VAT, LEI, EORI or DUNS identifier is stated as what it is. An identifier with
  no scheme is refused rather than defaulted.
- **Asynchronous validation** — Art. 8(7) has the registry rule on a submission
  after accepting it, so `RegistryStatusCode::Pending` is a normal outcome and
  not a success. `Deactivated` is likewise distinct from `Rejected`.

### Still divergent — blocked on the published API specification

- **Authentication** — `EuRegistryEnvelope` anticipates a bearer-token
  mechanism; registration rests on eIDAS verified-operator identity instead
  (Arts. 4–5). This is a structural mismatch, not a wrong endpoint, and it is
  the largest remaining gap.
- **Endpoint paths and API version** — the registry hosts are the Commission's
  published ones; the resource paths beneath them and `api_version` are not
  specified anywhere we can read, and remain our own construction.
- **Proof of registration** (Art. 9) — a secure electronic document, valid 90
  calendar days with regeneration, retrievable on request. No type models it,
  because the retrieval contract is unspecified.
- **Payload and envelope shape** — the field names and nesting are a reading of
  what Art. 8 requires to be registered, not a transcription of a schema.

Do not treat the shapes in the second list as an implementation target.

## Transfer-of-Responsibility Article Pin

**Superseded in part on 2026-08-26 by Implementing Regulation (EU) 2026/1778.**
The 2026-07-04 entry below concluded that no numbered transfer obligation
exists. That was correct as to Regulation (EU) 2024/1781 and remains so, but the
registry implementing act published **13 days later** supplies one at a
different level. Both halves are kept here: the correction first, the original
finding after it, because the original is still the answer for ESPR itself.

### The correction — IR 2026/1778 Art. 6a (verified 2026-08-26)

Verified against the OJ text (OJ L, 17.7.2026; in force on the twentieth day
following publication, i.e. 6 August 2026):

> **Article 6a — Transfer of registered digital product passports**
>
> "Registered digital product passports may be transferred to another verified
> economic operator or, where applicable, to a verified value chain actor that
> takes over the obligations from the previous actor in relation to those
> digital product passports from the date indicated for the transfer."

**Recital (10)** gives the triggers: an actor that "did not pass the identity
verification before the deadline", and "organisational changes such as merging,
splitting or sale of all or parts of the actor, cessation of activities or other
circumstances."

Three things follow, and each narrows what our `transfer` module may claim:

1. **What moves is the *registration*, not the passport document.** Art. 6a is
   about registered DPPs changing hands in the registry. It does not describe a
   mechanism for the passport artefact itself.
2. **The parties are eIDAS-*verified* actors**, under Arts. 4 and 5, and that
   status expires — Art. 5(4) caps it at "no longer than three years from the
   date of verification". A `did:web` identifier is not that, and a chain built
   on DIDs is not evidence of Art. 6a standing.
3. **The registry is the authority, and it is closed.** Art. 5(3): "Only
   verified value chain actors shall have access to the registry." So the
   registry can confirm who holds the obligations, but only to a reader who is
   themselves verified — never to the general public.

**Consequently the authoritative record of a transfer is the registry, and
`domain::transfer`'s chain is a local record of what this node was told and what
it notified upstream.** It is not, and must not be presented as, proof of who
holds the obligations. The node reports each completed handover upward to the
registry; that report is a projection of the local record, not a substitute for
the registry's own determination.

### The two mechanisms are distinct, and `TransferReason` spans both

Art. 6a is not the only way responsibility moves, and it does not cover the
product-lifecycle cases:

- **ESPR Art. 11(d)**: "where a **new** digital product passport is created for
  a product that already has a digital product passport, the new digital product
  passport shall be **linked** to the original digital product passport or
  passports."
- **ESPR Art. 2(16)** defines remanufacturing as "actions through which a **new
  product** is produced from objects that are waste, products or components and
  through which at least one change is made that substantially affects the
  safety, performance, purpose or type of the product."

Read together: a remanufactured product is a new product, it gets a **new,
linked** passport, and no transfer of the original occurs. That mechanism is
`Passport::derived_from`, not `TransferChain`.

So `TransferReason::Remanufacturing`, `::Repurposing`, `::PreparationForReuse`
and `::PreparationForRepurposing` sit closer to Art. 11(d) lineage than to
Art. 6a, while `::InsolvencySuccession` — and a *corporate* reading of `::Sale`,
meaning sale of the actor rather than of the product — sit under Art. 6a.
**Whether those four variants should exist at all is an open domain question,
deliberately not resolved by this entry.**

`::PreparationForRepurposing` was added after this entry was written, and does
not narrow the question. The four are the operations Reg. (EU) 2023/1542
Art. 77(7) names, and that article — unlike ESPR — says of exactly those four
that "the responsibility ... shall be transferred", which is the reason the set
is now complete rather than three-quarters of an article. The variant was added
so that whichever way the question falls, all four fall together; a set missing
one member could only ever be wrong.

Note also that **Art. 11(c)** and **Art. 11(e)** both anchor the obligation to
"the economic operator responsible for the **creation** of the digital product
passport", not to a moving current holder — which is why `operator_identifier`
is frozen at publish and is not rewritten by a transfer.

### The original finding — Regulation (EU) 2024/1781 (verified 2026-07-04)

Verified against the OJ text on 2026-07-04, to resolve an internal citation
ambiguity (the transfer-of-responsibility obligation had been cited
inconsistently as either Art. 9 or Art. 12). **This still stands as to ESPR.**

- **No single article of Regulation (EU) 2024/1781 establishes a
  transfer-of-responsibility mechanism** for a DPP moving between economic
  operators (resale, recycler take-over, insolvency succession, etc.).
- **Art. 11(e)** is the closest fit: it requires the passport to "remain
  available ... including after an insolvency, a liquidation or a cessation
  of activity ... of the economic operator responsible for the creation of
  the digital product passport" — a continuity/availability obligation, not
  a transfer-mechanics one.
- **Art. 10(4)** is the adjacent back-up-copy obligation (via a DPP service
  provider), already cited above.
- **Art. 9** establishes no transfer *mechanism*, but it is not silent
  either: alongside the placing-on-market gate, **Art. 9(1)** requires that
  passport data "shall be accurate, complete and up to date" — the standing
  duty that makes a stale post-transfer passport non-compliant. That duty,
  together with the registry-upload duty (**Art. 13(4)**), is the narrow
  basis cited in `docs/regulatory/CONFORMITY.md` and
  `dpp-registry`'s `transfer` module; it stands and is not superseded here.
- **Art. 12** (unique-*identifier* issuance mechanics, not registry upload)
  does not address transfer at all — the "Art. 12" leg of the earlier
  "Art. 9/12" citation was not traceable to operative text and is superseded
  by this entry.

### What `domain::transfer` may be described as

The two-step handshake is **an engineering design choice** that satisfies —
and exceeds — Art. 11(e)'s continuity requirement, and that produces the record
this node notifies to the registry under Art. 6a. ("Two-step", not "dual-signed":
two JWS values exist, but only `from_signature` is a counterparty's own. The
other is this node's attestation that acceptance ran — see
`TransferRecord::node_acceptance_attestation`, and §5.1 of
`docs/architecture/PRODUCT-LINEAGE.md` for a design that read the old wording the
wrong way.) It is **not** a literal
implementation of a numbered transfer obligation in ESPR, because none exists
there; and it is **not** evidence of an Art. 6a transfer, because that rests on
verified-actor status this node cannot attest to.

Treat this as engineering due diligence, not legal advice — verify
independently before relying on it in a filing or contract.

## Transparency Commitments

1. **All compliance-relevant code is open-source** under Apache-2.0.
2. **Schema validation is deterministic** — the same input always produces the
   same validation result, regardless of platform or runtime.
3. **No vendor lock-in** — dpp-core has zero infrastructure dependencies. Any
   platform that implements the port traits can use it.
4. **Audit trail** — every compliance rule change is tracked in version
   control with a link to the originating regulation.

## References

- [ESPR Regulation (EU) 2024/1781](https://eur-lex.europa.eu/eli/reg/2024/1781)
- [Battery Regulation (EU) 2023/1542](https://eur-lex.europa.eu/eli/reg/2023/1542)
- [DPP Registry Implementing Regulation (EU) 2026/1778](https://eur-lex.europa.eu/eli/reg_impl/2026/1778/oj)
- [CBAM Regulation (EU) 2023/956](https://eur-lex.europa.eu/eli/reg/2023/956)
- [GS1 Digital Link Standard](https://www.gs1.org/standards/gs1-digital-link)
- [W3C Verifiable Credentials Data Model v2.0](https://www.w3.org/TR/vc-data-model-2.0/)
- [SECURITY.md](../../SECURITY.md)
- [VERSIONING.md](../governance/VERSIONING.md)
