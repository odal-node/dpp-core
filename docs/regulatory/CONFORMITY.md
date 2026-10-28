# Conformity statement

## Purpose

This document records how `dpp-core` aligns with the EU Ecodesign for Sustainable Products
Regulation (ESPR, Regulation (EU) 2024/1781) and the expected product group delegated
acts. It is written for conformity assessment bodies and for anyone evaluating the library
against those texts.

## Regulatory references

| Reference | Status | `dpp-core` alignment |
|---|---|---|
| ESPR (EU) 2024/1781 | In force | The core data model follows Arts. 8 to 13 |
| CEN/CLC JTC 24 system standards | Six cited in the Official Journal on 15 July 2026 by CID (EU) 2026/1736: EN 18216, 18219, 18220, 18221, 18222, 18223 | **No conformance claimed.** No clause-by-clause assessment is published here. The ESPR Art. 41(2) presumption attaches to the cited standards, and claiming it needs a published assessment, not only a citation (which now exists) |
| EU Battery Regulation 2023/1542 | In force | `BatteryData` implements the Annex XIII fields (Art. 77 battery passport) |
| Textile delegated act | Pending; an ESPR working-plan priority | `TextileData` is held provisional and validated structurally until the act is adopted |

Technical specifications are recorded in
[`architecture/STANDARDS.md`](../architecture/STANDARDS.md): IETF, W3C, the GS1 Digital Link
URI Syntax (read against 1.7.0, while EN 18219:2026 names 1.6.0:2022; see
[GS1 interoperability](#gs1-interoperability)), the IDTA AAS metamodel and ETSI. Each row
gives the revision cited, its status when last read, the evidence behind the
implementation, and whether conformance is claimed.

## Access model: an Art. 77(2) lattice, not a ranking

Regulation (EU) 2023/1542 Art. 77(2) assigns three **audiences** to the four Annex XIII data
sets. Read against the Official Journal text, the assignment is not an ordering:

| Audience | Annex XIII points | Content |
|---|---|---|
| (a) General public | 1 | Public model-level information. No credential. |
| (b) Notified bodies, market surveillance authorities, the Commission | 2 and 3 | Composition, dismantling and safety, **plus** conformity test reports. |
| (c) Persons with a legitimate interest | 2 and 4 | The same point-2 data, **plus** individual-item data: state of health, use history, status. |

Point 3 is for authorities only and point 4 for legitimate interest only, so **neither
audience contains the other**. No integer "tier" comparison can express this: any `>=`
test either discloses to authorities the individual-item data Art. 77(2)(b) withholds, or
hides point-2 data from someone entitled to it. The implementation therefore models two
independent types, `Audience` (who is asking) and `Disclosure` (how restricted a field
is), related by one total function, `Audience::may_see(Disclosure)`.

ESPR itself does not fix a set of access levels. Read against the Official Journal text of
Regulation (EU) 2024/1781, three provisions carry this:

- **Art. 9(2)(f):** the delegated act specifies "the actors that are to have access to data
  in the digital product passport and to what data they are to have access".
- **Art. 10(1)(g):** access "shall be regulated in accordance with the essential
  requirements set out in this Article and Article 11 and with the specific access rights
  at product group level as specified in the applicable delegated act".
- **Art. 11(b):** the listed actors "shall have free of charge and easy access to the
  digital product passport based on their respective access rights set out in the
  applicable delegated act".

So the access lattice is set per product group, not by ESPR. Other product groups reuse the
same vocabulary through each manifest's `disclosure` map rather than inheriting a
hard-coded ladder.

> **Superseded.** Releases up to and including 0.10.0 implemented an ordered three-tier
> model (`AccessTier::{Public, Professional, Confidential}`). It was removed in 0.11.0 for
> the reason above. Assessments made against the earlier model should be re-read against
> this section.

### Implementation

- `dpp_domain::disclosure`: `Audience`, `Disclosure`, and `PASSPORT_FIELD_DISCLOSURE`, the
  single source for the disclosure class of every non-public top-level passport field.
- `dpp_vc::credential`: W3C VC issuance and verification, mapping an operator role to an
  `Audience`. A credential establishes *which* audience a caller holds.
- `dpp_domain::access::{policy, filter}`: a stateless policy engine that filters JSON fields
  against the caller's `Audience` and a `ProductGroupAccessPolicy`. It maps an audience to
  fields, which is a separate question from proving the audience, and lives with the
  passport because the disclosure classes are declared as data in the product group
  manifests.
- Integration test: `crates/dpp-tests/tests/access_gatekeeping.rs` exercises all three
  audiences with realistic credentials.

## Transfer of responsibility

ESPR has no article named "transfer of responsibility" (checked against the Official
Journal text of Regulation (EU) 2024/1781). This design follows from the general
data-accuracy duty (Art. 9(1)) and the registry-upload duty (Art. 13(4)). An earlier
"Art. 12" citation was wrong: Art. 12 is "Unique identifiers" (identifier issuance
mechanics). For batteries, Regulation (EU) 2023/1542 Art. 77(7) moves responsibility to the
operator placing on the market a battery that has undergone preparation for re-use,
preparation for repurposing, repurposing or remanufacturing. The
authoritative record of who holds the obligations is the EU registry, under Implementing
Regulation (EU) 2026/1778 Art. 6a; see [COMPLIANCE.md](COMPLIANCE.md).

The `dpp_domain::transfer` module implements:

- `TransferChain`: an append-only provenance log with state-machine validation.
- `ResponsibleOperator`: a DID-identified economic operator with a typed role.
- `TransferRecord`: a two-step transfer event. The outgoing operator signs the handover
  (`from_signature`); the hosting node records an attestation that the incoming operator
  accepted (`node_acceptance_attestation`). That attestation is not the incoming
  operator's signature.
- Integration test: `crates/dpp-tests/tests/transfer_of_responsibility.rs` covers the full
  lifecycle, error cases and serialisation round-trips.

## Schema validation

### Versioned schemas

All product group schemas are in `crates/dpp-domain/schemas/{product-group}/v{version}.json`
and follow JSON Schema Draft-07. `VersionedSchemaRegistry` embeds them at compile time with
`include_str!()`. That directory is the list of product groups and versions; it is not
repeated here, because the copy this section used to carry had stopped at four product
groups and their first versions.

### Textile field set

The textile schema's field set is this library's own. It is not derived from, and does not
claim to cover, any CEN/CLC JTC 24 standard: no clause-by-clause assessment of those
standards is published here (see the table above).

Integration test: `crates/dpp-tests/tests/schema_conformity.rs` protects the field set
against regression. Its header explains why that is not a conformity check.

## GS1 interoperability

- **Digital Link:** parsing and building the path of a GS1 Digital Link URI: any of GS1's
  sixteen primary keys with its qualifiers, read against GS1 Digital Link URI Syntax 1.7.0.
  The claim, its evidence and its deviations are in
  [`STANDARDS.md`](../architecture/STANDARDS.md). EN 18219:2026 clause 6.3.2 names
  **1.6.0:2022** as the revision its scheme 1 requires, which is a different revision; see
  below.
- **Link-type negotiation:** content negotiation returning different passport
  representations (JSON-LD, HTML, AAS) based on the `linkType` query parameter.
- **AAS submodel mapping:** conversion of passport JSON to AAS SubmodelElement structures,
  with this library's own semantics. No IDTA or Catena-X conformance is claimed.

## Unique identifier: ISO/IEC 15459 (Battery Regulation Art. 77(3))

Art. 77(3) of Regulation (EU) 2023/1542 requires that *"the QR code and the unique
identifier shall comply with ISO/IEC standards 15459-1:2014, 15459-2:2015, 15459-3:2014,
15459-4:2014, 15459-5:2014 and 15459-6:2014"*.

**Position.** The carrier is a GS1 Digital Link URI over a GS1 identification key: the GTIN
(AI 01), qualified at the level the passport describes. Nothing is added for a model, a
batch or lot (AI 10) for a batch, and a serial (AI 21) for an item or where no level is
stated, which makes it a serialised GTIN. **Unverified:** GS1 is a registered Issuing
Agency under ISO/IEC 15459, so identifiers issued under GS1 keys carry a registered Issuing
Agency Code and inherit the scheme's global-uniqueness guarantees. The ISO/IEC 15459 parts
have not been read here, so **no conformance to them is claimed**, through GS1 or otherwise.
A claim waits for a review of their text and for evidence that does not rest on GS1's own
statement, both recorded in [STANDARDS.md](../architecture/STANDARDS.md).

**What is verified.** A printed serial or lot is one to twenty GS1 CSET 82 characters (the
default serial is twenty characters from `[0-9a-f]`). `Passport::validate` and the builder
both enforce that against the vendored GS1 syntax dictionary's `X..20` for AI 10 and AI 21,
and the `DigitalLink` parser enforces it on the way back in. Tests cover it, and every CSET
82 membership decision in a serial and a lot is checked against GS1's Barcode Syntax Engine
through the oracle corpus.

**The URI syntax revision is 1.7.0, and EN 18219 names 1.6.0.** The parser and builder were
read against 1.7.0, which GS1 ratified in August 2026, and the register records what was run
and where this crate departs from it. EN 18219:2026 clause 6.3.2 names **1.6.0:2022** as the
revision its scheme 1 requires. GS1's change log for 1.7.0 lists five changes since 1.6.0:
data attributes for new Application Identifiers, harmonised terminology, editorial changes,
a new regular expression for compressed URIs, and the Barcode Syntax Resource named as a
validation tool. Whether a claim against 1.7.0 carries over to that clause depends on those
differences, which the harmonised-standard assessment has to settle. This document does not
claim it does.

**What is not verified.** The ISO/IEC 15459 parts are paywalled and have **not** been read
against their primary text. The position above rests on GS1's registration as an Issuing
Agency and on secondary sources, not on the standard's own wording. Do not restate it as a
verified conformance claim, and do not put it in customer-facing material, until someone
has read the parts, in particular 15459-3 (common rules) and 15459-4 (individual products),
which bear on a per-item product identifier.

**Serial construction.** The AI 21 serial is derived from the last ten bytes of the
passport's UUIDv7 (`rand_a` and `rand_b`, 74 random bits), not the first ten. The first six
bytes of a UUIDv7 are a millisecond timestamp: a serial derived from them increased
monotonically and its first twelve hex characters decoded to the passport's creation time,
so a QR code on a physical battery disclosed when it was created and, across several codes,
the production order and rate. Fixed; regression tests cover both properties.

**Open question.** ISO/IEC 15459 requires uniqueness to be *persistent over time*. The serial
is deterministic from the passport UUID and unique per passport, but nothing prevents two
passports being issued for one physical item, or a reissued passport getting a different
serial for the same battery. That is an operational guarantee a host must make; this crate
cannot enforce it.

## Processor limits: Art. 78(d)

Art. 78(d) of Regulation (EU) 2023/1542 forbids an operator acting on behalf of the
responsible economic operator from selling, re-using or processing passport data *"beyond
what is necessary for the provision of the relevant storing or processing services"*.

**It is met by architecture, not by a policy statement.** Every deployment serves a single
operator, with no shared cluster, so there is no surface on which one operator's passport
data and another's can be seen together. A cross-customer benchmark is not something the
system declines to build; it has nowhere to compute one.

**Where the constraint lives in code.** `PassportRepository` is the main persistence
surface (resolver scan telemetry is processed data too), and its `list` and `count` methods
are the only ones that see more than one passport. The port documents the Art. 78(d) limit
for implementers, so a future backing store cannot gain an analytics side line without
someone editing past the constraint.

**What it does not restrict.** An operator analysing its own passports. The prohibition
binds the processor acting for the operator, not the operator.

**Already applied.** Resolver scan telemetry records only per-passport, per-day,
per-variant counts, with no IP address, user agent or session identifier, because the
schema has no column for one. That design predates this section; Art. 78(d) is the article
it answers to.

**Remaining with your application.** `dpp-core` is stateless and holds no data, so it can
only state the constraint and place it at the port. Enforcement (log retention, backup
handling, what whoever runs the infrastructure may read) belongs to your application and its
infrastructure, and is not evidenced here.

## Personal data: ESPR Art. 10(1)(e)

ESPR Art. 10(1)(e) makes it an essential requirement that *"personal data relating to
customers shall not be stored in the digital product passport without their explicit
consent in compliance with Article 6 of Regulation (EU) 2016/679"*. Regulation (EU)
2025/2509 (toys) Art. 20(10) and Regulation (EU) 2026/405 (detergents) Art. 22(h) set the
same condition for their passports. Regulation (EU) 2023/1542 has no equivalent, so a battery
passport answers to Regulation (EU) 2016/679 (GDPR) and to its own Art. 78(h) requirement of
a high level of privacy.

**Personal data a passport is not required to carry is kept out of it.** Consent can be
withdrawn at any time (GDPR Art. 7(3)), and withdrawal obliges erasure where no other legal
ground remains (Art. 17(1)(b)). A published passport is signed, frozen, archived and copied,
so nothing inside it can be erased. Data the governing act requires, such as a battery's
Annex XIII point 4 usage record, is different: it rests on a legal obligation (GDPR
Art. 6(1)(c)), and erasure does not reach it (Art. 17(3)(b)).

**Where it lives in code.** Product group schemas mark operator-written free text that
describes one item's life after sale with `x-personal-data`.
`dpp_domain::check_personal_data`, run by `validate_passport`, refuses a marked field with a
value unless the passport's `personalData` states what is held about it: either nothing, or
an erasable record held outside the passport through `PersonalDataPort`, on a named GDPR
Art. 6(1) basis. Where a governing act admits customer personal data only with explicit
consent, any other basis is refused; that condition is recorded per act in the instrument
manifests. `redact_passport` shows a statement only to the audiences that see its field,
never to the public.

**What it does not do.** Read field content. Whether text is personal data cannot be decided
by a pattern, and the statement is the controller's answer, signed with its own key. Nor can
it check that a basis is valid or that consent was given. Those are for the controller to
establish and demonstrate (GDPR Art. 5(2), Art. 7(1)).

**When it binds.** Art. 10(1)(e) binds a product group once an ESPR delegated act requires
its passport, and none does yet. The toys and detergents passports apply from 1 August 2030
and 23 September 2029. GDPR applies now, to any personal data a passport or a draft holds,
whichever act governs it.

**Remaining with your application.** The check is a function, and your application decides
when it runs.
Where one of those acts applies, the prohibition is on *storing*, and a stored draft is
storage, so the check belongs on every write, not only at publish. Running it on every write
today costs nothing and keeps the passport within GDPR's minimisation principle before any of
those acts applies. Holding the records, erasing them on withdrawal, and keeping them out of
every served view are your application's job, through the port.

## Cryptographic foundations

- **Ed25519:** all signing uses Ed25519 (EdDSA). The curve is this library's choice; no
  provision of ESPR or its implementing acts is recorded as requiring it.
- **AES-256-GCM:** encryption of private keys at rest, with an Argon2id-derived key.
- **did:web:** the DID method for operator identification, with a DID document builder
  following W3C DID Core v1.0.
- **JWS (RFC 7515):** compact serialisation for passport and transfer signatures.

## Wasm plugin architecture

Product-group-specific compliance logic runs as sandboxed Wasm modules (`wasm32-wasip1`)
loaded by a host. The plugin ABI includes:

- Capability negotiation (plugins declare the operations they support).
- Semantic versioning with compatibility checks.
- A stateless contract: each call carries its whole input, and a plugin is a unit struct
  with no state of its own. Whether a host reuses a Wasm instance between calls is the
  host's decision and is not evidenced here.

## Test coverage

| Test suite | Location | Coverage |
|---|---|---|
| Textile end to end | `crates/dpp-tests/tests/textile_end_to_end.rs` | Passport lifecycle, AAS, GS1, credentials |
| Transfer of responsibility | `crates/dpp-tests/tests/transfer_of_responsibility.rs` | Transfer chain, provenance, error cases |
| Audience gatekeeping | `crates/dpp-tests/tests/access_gatekeeping.rs` | All three audiences, edge cases, custom policies |
| Schema conformity | `crates/dpp-tests/tests/schema_conformity.rs` | Schema validity and textile field-set regression; not a conformity check |
| Unit tests | Sibling `tests.rs` files beside each module | Every crate |

## CI gate

The `check` recipe in the `justfile` is the local gate, and its dependency list is the list
of what it runs: formatting, lints and tests for the workspace and the product group plugins,
doctests, the doc build, the lockfile check and `cargo audit`. It is not repeated step by
step here, because the four-step copy this section used to carry had fallen behind it.
GitHub Actions (`.github/workflows/`) covers the same ground in separate jobs and adds what
`check` cannot: the Wasm cross-compiles, the orphaned-tests guard, and the GS1, AAS and JAdES
oracle suites in their own workflows.

## Known gaps

1. **DID resolution.** `dpp_crypto::jws` verifies an EdDSA signature against a public key
   the caller supplies. Fetching a `did:web` document to obtain that key needs an HTTP
   client, which the library does not have, so your application does it.
2. **Status list fetching.** Revocation is decided against a W3C Bitstring Status List v1.0:
   `dpp_vc::status_list` decodes the list and `check_revocation` reads the credential's bit,
   failing closed on an index the list cannot answer. Fetching the status list credential
   needs an HTTP client, so your application supplies the list.
3. **Schema hot-reload** is implemented, but the file-watching trigger lives in your
   application.
4. **Wasm plugins** are outside the Cargo workspace. `just check` still formats, lints and
   tests them through their own recipes, and the `wasm-build.yml` workflow cross-compiles
   every plugin to `wasm32-wasip1`.

## Contact

For conformity assessment enquiries: dev@odal-node.io
