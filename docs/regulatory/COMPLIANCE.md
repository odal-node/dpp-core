# Regulatory compliance mapping

How `dpp-core` maps to the EU regulations it implements, which standards it follows, and
how regulatory changes are brought in.

## Regulatory scope

`dpp-core` provides the domain types and validation logic for Digital Product Passports
required by:

| Regulation | Reference | Status | Coverage in `dpp-core` |
|---|---|---|---|
| ESPR | Regulation (EU) 2024/1781 | In force | Core passport model, per-audience field disclosure, schema validation, the unsold-goods disclosure (Arts. 24 and 25) |
| Battery Regulation | Regulation (EU) 2023/1542 | In force; battery passport from 18 February 2027 | Battery schemas, product group plugin, per-category content rules |
| Textiles | ESPR delegated act, not adopted | See the note below | Provisional textile schemas, product group plugin |
| CBAM | Regulation (EU) 2023/956 | In force; not in the instrument catalog | The steel and aluminium schemas carry embedded-emissions fields aligned with its reporting |

> **Timeline note:** the textile delegated act has no adopted date. Published estimates
> range from about 2026 to 2028. Treat any specific year as provisional and check EUR-Lex
> before relying on it.

`dpp-core` does **not** implement infrastructure (HTTP APIs, databases, registry
connectivity). That belongs to your application.

## Standards alignment

| Standard | Body | Version tracked | Where it is used |
|---|---|---|---|
| GS1 Digital Link | GS1 | URI Syntax 1.7.0 (status in [`STANDARDS.md`](../architecture/STANDARDS.md)) | `dpp-digital-link`: URL parsing and link-type negotiation |
| Asset Administration Shell | IDTA | Metamodel 3.0 | `dpp-aas`: submodel mapping |
| W3C Verifiable Credentials | W3C | Data Model v2.0 | `dpp-vc`: credential issuance and verification |
| `did:web` | W3C CCG | did:web method specification | `dpp-vc`: DID document builder |
| JSON Schema | json-schema.org | Draft-07 | `dpp-domain`: passport data validation |
| CEN/CENELEC JTC 24 | CEN/CENELEC | Six EN 182xx:2026 standards cited in the Official Journal; two still drafts | A conformance target, not a source of terms. No clause text is reproduced here |

> **On the JTC 24 row.** Whether these standards are cited in the Official Journal, and so
> whether conformity to one carries a presumption of conformity, is a fact with one home:
> the `jtc24` record in `dpp-vocab`'s `vocabularies/`, with its source and the date it was
> checked. This table names the standard and does not restate that record. An earlier
> version of this row said "Draft (monitoring)" and was stale, which is why the two are
> split.

## Product group to regulation

Each product group's schemas are traceable to their regulatory basis. The current
versions below come from each manifest's `currentSchemaVersion` in
`crates/dpp-domain/product-groups/`, which is the authority; older versions stay embedded
so stored passports remain valid against the version they declare.

| Product group | Current schema | Act bound in the catalog | Binding status | Notes |
|---|---|---|---|---|
| `battery` | v2.10.0 | Regulation (EU) 2023/1542 | in force | Annex XIII points 1 to 4 and Annex VI Part A; per-property `x-disclosure` |
| `electronics` | v1.5.0 | Regulations (EU) 2023/1670 and 2023/1669 | in force | Phones and tablets. The passport duty is displaced to EPREL |
| `unsold-goods` | v2.0.0 | ESPR Arts. 24 and 25; Regulations (EU) 2026/2 and 2026/296 | in force | A yearly disclosure, not a passport |
| `textile` | v1.4.0 | ESPR | provisional | Fibre composition, durability, microplastics; no delegated act yet |
| `steel` | v1.2.0 | ESPR | provisional | Production route and embedded emissions, aligned with CBAM |
| `aluminium` | v1.2.0 | ESPR | provisional | Production route, CO₂e per tonne, recycled content |
| `furniture` | v1.3.0 | ESPR | provisional | |
| `mattress` | v1.1.0 | ESPR | provisional | Split from furniture by the ESPR working plan |
| `tyre` | v1.1.0 | ESPR | provisional | |
| `construction` | v1.2.0 | Regulation (EU) 2024/3110 (CPR) | provisional | |
| `detergent` | v1.2.0 | Regulation (EU) 2026/405 | provisional | Surfactant bands per its Annex VII |
| `toy` | v1.2.0 | Regulation (EU) 2025/2509 | provisional | |

The current version per product group is resolved by `ProductGroupCatalog`, not
hard-coded at call sites. [REGULATORY.md](REGULATORY.md) says which product groups carry
implemented compliance rules and which are placeholders.

## Regulatory change process

When an EU regulation or delegated act is published or amended:

1. **Monitor:** follow publications on EUR-Lex and the JTC 24 mailing lists.
2. **Assess:** open a GitHub issue labelled `regulatory` describing the change and its
   effect on schemas, domain types or compliance rules.
3. **Implement:** create a new schema version (never modify an existing one) and update
   domain types if needed. Every change goes through the normal pull request process.
4. **Document:** update the mapping tables here and add a CHANGELOG entry that cites the
   regulation.
5. **Release:** cut a new version following the [release process](../governance/RELEASE.md).

### Schema immutability

Published schema versions are **never modified**. If a regulation changes requirements, a
new version is created, so a passport validated against v1 stays valid against v1 after
v2 is published.

**When a citation in a published version turns out to be wrong**, that version keeps it
and the correction is recorded as a known erratum in the module documentation of
`dpp_domain::schemas`, beside the table that embeds the versions, so it reaches a reader
who vendored the crate from crates.io and has nothing else to consult. Three are recorded
today: `battery/v2.0.0`'s "Annex XIII §6", `battery/v2.6.0`'s Art. 13(4) marking
citations, and the `textile` `allergens` description in v1.1.0 and v1.2.0.

> **Errata cover field descriptions only.** A wrong *constant*, threshold or enumerated
> category is not an erratum. It is a defect that needs a new schema version, because
> something branches on it.

## Compliance architecture

`dpp-core` exposes one extension point for product-group-specific compliance:

```text
ComplianceRegistry (port trait)
  ├── PassthroughRegistry (this library)  → PassthroughNoValidation, computes nothing
  └── plugin-backed registry (a host)     → Wasm product group plugins (product-group-battery and others)
```

`ComplianceRegistry::compute(&self, product_group_key: &str, data: &ProductGroupData,
law_in_force_on: Option<NaiveDate>)` routes to a per-group `ComplianceStrategy`, whose
`compute(&self, data, law_in_force_on)` returns a `ComplianceResult`. The Apache-2.0
default (`PassthroughRegistry`) computes nothing and returns `PassthroughNoValidation` for
every product group, including one with no registered strategy, which gets
`ComplianceResult::passthrough()` rather than an error. `UnknownProductGroup` is for an
implementation that genuinely cannot serve a group; an implementation may return it, and
this one never does. Real determinations come from the Wasm product group plugins. A
computed determination passes through `gate_determination(in_force, …)`, where `in_force`
is whether `InstrumentCatalog::determinable_for(product_group)` returns any (act,
binding) pair, so a provisional product group can never produce a binding result. This
separation means:

- **The library stays generic:** no product-group-specific determination logic in the
  workspace crates.
- **Regulations are isolated:** a Battery Regulation change touches only
  `product-group-battery` and the shared rules in `dpp-rules`.
- **New product groups are additive:** a new delegated act means a new versioned schema
  file, a catalog manifest and a plugin. A typed data struct and its `ProductGroupData`
  variant follow; until they exist, the group's data is carried as
  `ProductGroupData::Other`.

## Cryptography

| Primitive | Algorithm | Library | Purpose | Regulatory basis |
|---|---|---|---|---|
| Signing | Ed25519 | `ed25519-dalek` | Passport authenticity (JWS) | ESPR Art. 11(g): data authentication, reliability and integrity |
| Keystore encryption | AES-256-GCM, Argon2id key derivation | `aes-gcm`, `argon2` | Private keys at rest | Key custody, so signatures can be trusted |
| Hashing | SHA-256 | `sha2` | Thumbprints, content digests, hash pins | Integrity |
| Entropy | OS CSPRNG | `rand` | Key generation | |

Access to restricted fields is enforced by disclosure filtering (Regulation (EU)
2023/1542 Art. 77(2)), not by encrypting fields. [SECURITY.md](../../SECURITY.md) and
[IDENTITY.md](../architecture/IDENTITY.md) describe the primitives.

## EU registry readiness

The EU DPP registry (ESPR Article 13) **became operational on 20 July 2026**, with a
testing environment and user guidelines. Its operating rules are Commission Implementing
Regulation (EU) 2026/1778.

Checked against the Official Journal text of IR 2026/1778 (adopted 16 July 2026,
published 17 July 2026, in force on the twentieth day after publication, **Art. 24**):

- **Who may register.** A passport is registered by a *verified economic operator*
  placing the product on the market (**Art. 8(1)**). Verification uses an eIDAS
  credential: for a legal person, a **qualified electronic seal** supported by a qualified
  certificate from a QTSP, or a qualified electronic attestation of attributes
  (**Art. 4(2)**). Establishment in the Union is **not** a precondition: Art. 4(2)(b)
  provides for operators not required to be established there.
- **Registration by a third party is permitted.** Where a verified economic operator
  authorises a third party to register on its behalf, that third party must itself
  complete the verification of **Art. 5** (value chain actors, which the recitals say
  include a digital product passport service provider). The economic operator "shall
  remain fully responsible for compliance with the obligations set out in this Regulation"
  (**Art. 19(4)**) and is the controller of the data it submits (**Art. 19(5)**).
  Delegating the mechanics does not move the liability.
- **Registry structure** (**Art. 3**): an API for registering passports and retrieving
  information (Art. 3(b)), a list of verified digital product passport service providers
  (Art. 3(f)), and a storage component for unique identifiers *and commodity codes for
  products intended to be placed under the customs procedure "release for free
  circulation"* (Art. 3(e)).
- **Granularity.** Registration happens at model, batch or item level as the applicable
  delegated act requires (**Art. 8(1)**); where rules conflict, at the most granular level
  required (**Art. 8(3)**). An item-level passport must link **both** batch and model
  identifiers where they exist (**Art. 8(4)**); a batch-level passport must link the model
  identifier (**Art. 8(5)**).
- **Automated checks on submission** (**Art. 8(7)**): semantic conformity, coherence of
  mandatory data, conformity with the required granularity level and, *where relevant*,
  validity of the commodity code. `RegistrationPayload::check_coherence` runs the coherence
  and level checks (points (b) and (c)) against the passport, over the fields this crate
  registers. No binding specification of which data the registry treats as mandatory has
  been published for any product group yet.
- **Transfer of registered passports** (**Art. 6a**): a registered passport "may be
  transferred to another verified economic operator or, where applicable, to a verified
  value chain actor that takes over the obligations from the previous actor ... from the
  date indicated for the transfer." Recital (10) gives the triggers: failed identity
  verification and organisational change (merger, split, sale of the actor, cessation).
  **The registry is therefore the authoritative record of who holds the obligations, and
  Art. 5(3) closes it to everyone except verified actors.** The section on the
  transfer-of-responsibility article below covers how this limits what `domain::transfer`
  may claim.

The `dpp-registry` crate holds interface types first written before this specification was
published, and since reconciled against it where the Official Journal text fixes the answer:

- `RegistrationPayload`, `EuRegistryEnvelope`, `EuRegistryResponse`, `StatusResponse`,
  `TransferNotification`, the four Art. 13 identifier structs (`ProductIdentifier`,
  `ProductItemIdentifier`, `FacilityIdentifier`, `OperatorIdentifier`), error types, and
  `RegistryEndpoint`.
- `RegistrySyncPort`: the port trait (in `dpp_domain::ports`, with the `GhostRegistrySync`
  stub) that a host implements.

These types remain unstable. What the Official Journal text *fixes* has been reconciled;
what only the API specification can fix has not, and is listed separately so the two are
never confused.

### Reconciled against the Official Journal text (0.16.0)

- **Registration granularity:** `RegistrationLevel` carries the model, batch or item level
  (Art. 8(1)) and the identifiers Art. 8(4) and 8(5) require it to link. A missing model
  or batch identifier is treated as lawful, because both obligations depend on such a
  design existing, and recital (14) says products unique by nature have neither. `item_id`
  depends on the declared level rather than being unconditional.
- **Commodity code:** `CommodityCode` on the passport and the registration payload,
  validated *structurally* as HS-6, CN-8 or TARIC-10. Whether a code falls inside the range
  a product group permits (Art. 8(7)(d)) is **not** checked here: those ranges live in the
  applicable delegated act. Absence remains lawful, because the obligation applies "where
  relevant".
- **Operator identifier scheme:** carried explicitly rather than assumed, so a VAT, LEI,
  EORI or DUNS identifier is stated as what it is. An identifier with no scheme is refused,
  not defaulted.
- **Asynchronous validation:** Art. 8(7) has the registry rule on a submission after
  accepting it, so `RegistryStatusCode::Pending` is a normal outcome and not a success.
  `Deactivated` is likewise distinct from `Rejected`.

### Still open, waiting for the published API specification

- **Authentication** has two layers that must not be confused. A token in front of the
  API fits the act: **Art. 2(3)** defines an *authentication token* for sessions and
  delegated access between applications and the registry, and **Art. 18(1)(b)** stores
  tokens as user credentials. Identity is the verified status of **Arts. 4 and 5**, proved
  by qualified electronic seal or attestation, and a token proves none of it. How a token
  is obtained is unpublished, so `RegistryEndpoint::token_endpoint` and any exchange
  performed against it are our own construction. This is the largest remaining gap.
- **Endpoint paths and API version:** the hosts are those the registry's own interface
  calls, read from its runtime configuration on 2026-10-14: the API on
  `reg.product-passport.ec.europa.eu` (and `reg.acc.…` for the test environment), which is
  not the published `registry.…` address the interface is served from. The registration,
  product-group and dashboard task routes are observed in the same client; the status and
  transfer routes and `api_version` are not specified anywhere we can read, and remain our
  own construction.
- **Proof of registration** (Art. 9): `ProofOfRegistration` models the Art. 9(2) content
  and the Art. 9(4) ninety days, which is how long the registry keeps a proof available,
  not how long it is valid. The retrieval call is unspecified, so nothing fetches one.
- **Payload and envelope shape:** the field names and nesting are a reading of what Art. 8
  requires to be registered, not a transcription of a schema. The registry's guide
  describes the outcome of a submission per passport (the unique product identifier with
  its registration identifier, or with its error), so `SubmissionReceipt` matches results
  by identifier; the field names are ours.

Do not treat the shapes in the second list as an implementation target.

## The transfer-of-responsibility article

**Partly superseded on 2026-08-26 by Implementing Regulation (EU) 2026/1778.** The
2026-07-04 entry below concluded that no numbered transfer obligation exists. That was
correct for Regulation (EU) 2024/1781 and still is, but the registry implementing act,
published **13 days later**, supplies one at a different level. Both are kept here, the
correction first, because the original is still the answer for ESPR itself.

### The correction: IR 2026/1778 Art. 6a (checked 2026-08-26)

Checked against the Official Journal text (OJ L, 17.7.2026; in force on the twentieth day
after publication, 6 August 2026):

> **Article 6a: Transfer of registered digital product passports**
>
> "Registered digital product passports may be transferred to another verified
> economic operator or, where applicable, to a verified value chain actor that
> takes over the obligations from the previous actor in relation to those
> digital product passports from the date indicated for the transfer."

**Recital (10)** gives the triggers: an actor that "did not pass the identity verification
before the deadline", and "organisational changes such as merging, splitting or sale of
all or parts of the actor, cessation of activities or other circumstances."

Three things follow, and each narrows what the `transfer` module may claim:

1. **What moves is the *registration*, not the passport document.** Art. 6a is about
   registered passports changing hands in the registry. It describes no mechanism for the
   passport artefact itself.
2. **The parties are eIDAS-*verified* actors** under Arts. 4 and 5, and that status
   expires: Art. 5(4) caps it at "no longer than three years from the date of
   verification". A `did:web` identifier is not that, and a chain built on DIDs is not
   evidence of Art. 6a standing.
3. **The registry is the authority, and it is closed.** Art. 5(3): "Only verified value
   chain actors shall have access to the registry." The registry can confirm who holds the
   obligations, but only to a reader who is verified, never to the general public.

**So the authoritative record of a transfer is the registry, and `domain::transfer`'s chain
is a local record of what a node was told and what it reported upstream.** It is not proof
of who holds the obligations and must not be presented as such. A node reports each
completed handover to the registry; that report is a projection of the local record, not a
substitute for the registry's own determination.

### The two mechanisms are distinct, and `TransferReason` spans both

Art. 6a is not the only way responsibility moves, and it does not cover the product
lifecycle cases:

- **ESPR Art. 11(d):** "where a **new** digital product passport is created for a product
  that already has a digital product passport, the new digital product passport shall be
  **linked** to the original digital product passport or passports."
- **ESPR Art. 2(16)** defines remanufacturing as "actions through which a **new product** is
  produced from objects that are waste, products or components and through which at least
  one change is made that substantially affects the safety, performance, purpose or type of
  the product."

Read together: a remanufactured product is a new product, it gets a **new, linked**
passport, and no transfer of the original occurs. That mechanism is
`Passport::derived_from`, not `TransferChain`.

So `TransferReason::Remanufacturing`, `::Repurposing`, `::PreparationForReuse` and
`::PreparationForRepurposing` sit closer to Art. 11(d) lineage than to Art. 6a, while
`::InsolvencySuccession`, and a *corporate* reading of `::Sale` (sale of the actor rather
than of the product), sit under Art. 6a. **Whether those four variants should exist at all
is an open domain question that this entry deliberately leaves unresolved.**

`::PreparationForRepurposing` was added after this entry was written and does not narrow
the question. The four are the operations Reg. (EU) 2023/1542 Art. 77(7) names, and that
article, unlike ESPR, says of exactly those four that "the responsibility ... shall be
transferred", which is why the set is now complete. The variant was added so that
whichever way the question falls, all four fall together.

**Art. 11(c)** and **Art. 11(e)** both anchor the obligation to "the economic operator
responsible for the **creation** of the digital product passport", not to a moving current
holder, which is why `operator_identifier` is frozen at publish and not rewritten by a
transfer.

### The original finding: Regulation (EU) 2024/1781 (checked 2026-07-04)

Checked against the Official Journal text on 2026-07-04, to resolve an internal citation
ambiguity (the transfer-of-responsibility obligation had been cited as either Art. 9 or
Art. 12). **This still stands for ESPR.**

- **No single article of Regulation (EU) 2024/1781 establishes a transfer-of-responsibility
  mechanism** for a passport moving between economic operators (resale, take-over by a
  recycler, insolvency succession and so on).
- **Art. 11(e)** is the closest: the passport must "remain available ... including after an
  insolvency, a liquidation or a cessation of activity ... of the economic operator
  responsible for the creation of the digital product passport". That is a continuity
  obligation, not a transfer mechanism.
- **Art. 10(4)** is the adjacent back-up copy obligation (through a passport service
  provider), cited above.
- **Art. 9** establishes no transfer *mechanism*, but it is not silent: besides the
  placing-on-market gate, **Art. 9(1)** requires that passport data "shall be accurate,
  complete and up to date", the standing duty that makes a stale passport after a transfer
  non-compliant. That duty, with the registry-upload duty (**Art. 13(4)**), is the narrow
  basis cited in `docs/regulatory/CONFORMITY.md` and in `dpp-registry`'s `transfer` module,
  and it stands.
- **Art. 12** (mechanics of unique *identifier* issuance, not registry upload) does not
  address transfer at all. The "Art. 12" half of the earlier "Art. 9/12" citation was not
  traceable to operative text and is superseded by this entry.

### How `domain::transfer` may be described

The two-step handshake is **an engineering design choice**. It satisfies, and goes beyond,
Art. 11(e)'s continuity requirement, and it produces the record a node reports to the
registry under Art. 6a. It is "two-step", not "dual-signed": two JWS values exist, but only
`from_signature` is a party's own. The other is the hosting node's attestation that acceptance ran;
see `TransferRecord::node_acceptance_attestation`, and §5.1 of
`docs/architecture/PRODUCT-LINEAGE.md` for a design that read the old wording the wrong way.
It is **not** a literal implementation of a numbered transfer obligation in ESPR, because
none exists there, and it is **not** evidence of an Art. 6a transfer, because that rests on
verified-actor status a node cannot attest to.

This is engineering due diligence, not legal advice. Check it independently before relying
on it in a filing or contract.

## Commitments

1. **All compliance-relevant code is open source** under Apache-2.0.
2. **Schema validation is deterministic:** the same input always gives the same validation
   result, on any platform or runtime.
3. **No lock-in:** `dpp-core` has no infrastructure dependencies, and any application that
   implements the port traits can use it.
4. **Traceable changes:** every compliance rule change is in version control, with a link
   to the regulation behind it.

## References

- [ESPR, Regulation (EU) 2024/1781](https://eur-lex.europa.eu/eli/reg/2024/1781)
- [Battery Regulation, Regulation (EU) 2023/1542](https://eur-lex.europa.eu/eli/reg/2023/1542)
- [DPP registry, Implementing Regulation (EU) 2026/1778](https://eur-lex.europa.eu/eli/reg_impl/2026/1778/oj)
- [CBAM, Regulation (EU) 2023/956](https://eur-lex.europa.eu/eli/reg/2023/956)
- [GS1 Digital Link](https://www.gs1.org/standards/gs1-digital-link)
- [W3C Verifiable Credentials Data Model v2.0](https://www.w3.org/TR/vc-data-model-2.0/)
- [SECURITY.md](../../SECURITY.md)
- [VERSIONING.md](../governance/VERSIONING.md)
