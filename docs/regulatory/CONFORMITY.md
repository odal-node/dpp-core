# Conformity Statement

## Purpose

This document records the regulatory alignment of `dpp-core` with the EU
Ecodesign for Sustainable Products Regulation (ESPR, Regulation (EU)
2024/1781) and the anticipated product group delegated acts. It is intended for
conformity assessment bodies and for anyone evaluating the library against
those texts.

## Regulatory References

| Reference | Status | dpp-core Alignment |
|---|---|---|
| ESPR (EU) 2024/1781 | In force | Core data model follows Art. 8–13 requirements |
| CEN/CLC JTC 24 system standards | Six cited in the OJ on 15 Jul 2026 by CID (EU) 2026/1736: EN 18216, 18219, 18220, 18221, 18222, 18223 | **No conformance claimed.** No clause-by-clause assessment is published here. The ESPR Art. 41(2) presumption attaches to the cited standards, and claiming it requires an assessment we have not published — not merely a citation, which now exists |
| EU Battery Regulation 2023/1542 | In force | `BatteryData` struct implements Annex XIII fields (Art. 77 battery passport) |
| Textile DPP Delegated Act | Pending — an ESPR working-plan priority | `TextileData` struct held provisional; validated structurally until the act finalises |

Technical specifications are recorded in
[`architecture/STANDARDS.md`](../architecture/STANDARDS.md): IETF, W3C, the GS1
Digital Link URI Syntax (whose implemented revision is not established; see
[GS1 Interoperability](#gs1-interoperability)), the IDTA AAS metamodel and ETSI.
Each row gives the revision cited, its status as last read, the evidence behind
the implementation, and whether conformance is claimed.

## Access Model — an Art. 77(2) lattice, not a ranking

Regulation (EU) 2023/1542 Art. 77(2) assigns three **audiences** to the four
Annex XIII data sets. Read against the verbatim OJ text, the assignment is not
an ordering:

| Audience | Annex XIII points | Content |
|---|---|---|
| (a) General public | 1 | Public model-level information. No credential. |
| (b) Notified bodies, market surveillance authorities, the Commission | 2 and 3 | Composition, dismantling and safety, **plus** conformity test reports. |
| (c) Persons with a legitimate interest | 2 and 4 | The same point-2 data, **plus** individual-item data: state of health, use history, status. |

Point 3 is authority-only and point 4 is legitimate-interest-only, so **neither
audience contains the other**. No integer "tier" comparison can express this: any
`>=` test necessarily either discloses to authorities the individual-item data
Art. 77(2)(b) withholds, or hides point-2 data from someone entitled to it. The
implementation therefore models two independent types — `Audience` (who is
asking) and `Disclosure` (how restricted a field is) — related by a single
total function, `Audience::may_see(Disclosure)`.

ESPR does not itself fix a set of access levels. Read against the verbatim OJ
text of Regulation (EU) 2024/1781, three provisions carry the point:

- **Art. 9(2)(f)** — the delegated act specifies "the actors that are to have
  access to data in the digital product passport and to what data they are to
  have access".
- **Art. 10(1)(g)** — access "shall be regulated in accordance with the
  essential requirements set out in this Article and Article 11 and with the
  specific access rights at product group level as specified in the applicable
  delegated act".
- **Art. 11(b)** — the listed actors "shall have free of charge and easy access
  to the digital product passport based on their respective access rights set
  out in the applicable delegated act".

So the access lattice is set per product group, not by ESPR itself. Non-battery
product groups therefore reuse this same vocabulary through each product group manifest's
`disclosure` map rather than inheriting a hardcoded ladder.

> **Superseded.** Releases up to and including 0.10.0 implemented an ordered
> three-tier model (`AccessTier::{Public, Professional, Confidential}`). It was
> removed in 0.11.0 for the reason above. Assessments performed against the
> earlier model should be re-read against this section.

### Implementation

- `dpp_domain::disclosure` — `Audience`, `Disclosure`, and
  `PASSPORT_FIELD_DISCLOSURE`, the single source for the disclosure class of
  every non-public top-level passport field.
- `dpp_vc::credential` — W3C VC issuance and verification, mapping an operator
  role to an `Audience`. A credential establishes *which* audience a caller
  holds.
- `dpp_domain::access::{policy, filter}` — stateless policy engine that filters
  JSON fields against the caller's `Audience` and a `ProductGroupAccessPolicy`. This
  maps an audience to fields, which is a separate question from proving the
  audience, and lives with the passport because the disclosure classes are
  declared as data in the product group manifests.
- Integration test: `crates/dpp-tests/tests/access_gatekeeping.rs` exercises all
  three audiences with realistic credentials.

## Transfer of Responsibility

No distinct "transfer of responsibility" article exists in ESPR by that name (checked against the
verbatim OJ text of Regulation (EU) 2024/1781); this design follows from the general data-accuracy
duty (Art. 9(1)) and the registry-upload duty (Art. 13(4)), not a single dedicated article. The
prior "Art. 12" citation was wrong — Art. 12 is "Unique identifiers" (operator/facility identifier
issuance mechanics).

When a product undergoes remanufacturing, repurposing, or preparation for
reuse, the new economic operator assumes full DPP responsibility. The
`dpp-domain::transfer` module implements:

- `TransferChain` — Append-only provenance log with state machine validation.
- `ResponsibleOperator` — DID-identified economic operator with role typing.
- `TransferRecord` — Dual-signature transfer event (JWS from both parties).
- Integration test: `crates/dpp-tests/tests/transfer_of_responsibility.rs` covers full
  lifecycle, error cases, and serialisation round-trips.

## Schema Validation

### Versioned Schemas

All product group schemas reside in
`crates/dpp-domain/schemas/{product-group}/v{version}.json` and follow JSON
Schema Draft-07. The `VersionedSchemaRegistry` embeds them at compile time via
`include_str!()`. That directory is the list of product groups and versions; it
is not restated here, because the copy this section used to carry had stopped
at four product groups and their first versions.

### Textile Field Set

The textile schema's field set is this library's own. It is not derived from,
and is not claimed to cover, any CEN/CLC JTC 24 standard — no clause-by-clause
assessment of those standards is published here (see the table above).

Integration test: `crates/dpp-tests/tests/schema_conformity.rs` holds the field
set against regression. Its own header says why that is not a conformity
check.

## GS1 Interoperability

- **Digital Link** — Parsing and building the path of a GS1 Digital Link URI:
  any of GS1's sixteen primary keys with its qualifiers, read against GS1 Digital
  Link URI Syntax 1.7.0. The claim, its evidence and its deviations are in
  [`STANDARDS.md`](../architecture/STANDARDS.md). EN 18219:2026 clause 6.3.2
  names **1.6.0:2022** as the version its scheme 1 requires, which is a different
  revision; see below.
- **Link-type Negotiation** — Content negotiation returning different DPP
  representations (JSON-LD, HTML, AAS) based on the `linkType` query parameter.
- **AAS Submodel Mapping** — Conversion of passport JSON to AAS
  SubmodelElement structures, carrying this library's own semantics. No IDTA or
  Catena-X conformance is claimed.

## Unique Identifier — ISO/IEC 15459 (Battery Reg. Art. 77(3))

Art. 77(3) of Regulation (EU) 2023/1542 requires that *"the QR code and the
unique identifier shall comply with ISO/IEC standards 15459-1:2014,
15459-2:2015, 15459-3:2014, 15459-4:2014, 15459-5:2014 and 15459-6:2014"*.

**Position.** The carrier is a GS1 Digital Link URI over a GS1 identification
key: the GTIN (AI 01), qualified at the level the passport describes — nothing
for a model, a batch/lot (AI 10) for a batch, and a serial (AI 21) for an item
or where no level is stated, which makes it a serialised GTIN. 🔶 GS1 is a
registered Issuing Agency under ISO/IEC 15459, so identifiers issued under GS1
keys carry a registered Issuing Agency Code and inherit the scheme's
global-uniqueness guarantees. Conformance is therefore claimed **through GS1**,
not by independent implementation of the ISO parts.

**What is verified.** A printed serial or lot is one to twenty GS1 CSET 82
characters — the default serial is twenty characters from `[0-9a-f]` — which
`Passport::validate` and the builder both enforce against the vendored GS1
syntax dictionary's `X..20` for AI 10 and AI 21, and which the `DigitalLink`
parser enforces on the way back in. That is covered by tests, and every CSET 82
membership decision in a serial and in a lot is judged by GS1's Barcode Syntax
Engine through the oracle corpus.

**The URI syntax revision is 1.7.0, and EN 18219 names 1.6.0.** The parser and
builder were read against 1.7.0, which GS1 ratified in August 2026, and the
register records what was run and where this crate departs from it.
EN 18219:2026 clause 6.3.2 names **1.6.0:2022** as the revision its scheme 1
requires. GS1's change log for 1.7.0 lists five changes since 1.6.0: data
attributes added for new Application Identifiers, harmonised terminology,
editorial changes, a new regular expression for compressed URIs, and the Barcode
Syntax Resource named as a validation tool. Whether a claim made against 1.7.0
carries over to that clause depends on those differences, and that is for the
harmonised-standard assessment to settle. This document does not say it does.

**What is not.** 🔶 The ISO/IEC 15459 parts are paywalled and have **not** been
read against primary text. The claim above rests on GS1's registration as an
Issuing Agency and on secondary sources, not on the standard's own wording. Do
not restate it as a verified conformance assertion, and do not put it in
customer-facing material, until someone has read the parts — in particular
15459-3 (common rules) and 15459-4 (individual products), which are the two that
bear on a per-item product identifier.

**Serial construction.** The AI 21 serial is derived from the passport UUIDv7's
last ten bytes (`rand_a` + `rand_b`, 74 random bits), not its first ten. The
leading six bytes of a UUIDv7 are a millisecond timestamp: deriving the serial
from them produced a monotonically increasing serial whose first twelve hex
characters decoded to the passport's creation instant, so a QR code on a
physical battery disclosed when it was created and, across several codes, the
production order and rate. Fixed; regression tests cover both properties.

**Open question.** ISO/IEC 15459 requires uniqueness to be *persistent over
time*. The serial is deterministic from the passport UUID and unique per
passport, but nothing currently prevents two passports being issued for one
physical item, or a reissued passport receiving a different serial for the same
battery. That is an operational guarantee a host must make, not one this
crate can enforce.

## Processor Limits — Art. 78(d)

Art. 78(d) of Regulation (EU) 2023/1542 forbids an operator authorised to act on
behalf of the responsible economic operator from selling, re-using or processing
passport data *"beyond what is necessary for the provision of the relevant
storing or processing services"*.

**What satisfies it is architectural, not a policy promise.** Every deployment is
single-operator — one node per operator, self-hosted or hosted, with no shared
cluster — so no surface exists on which one customer's passport data and
another's can be seen together. A cross-customer benchmark is not something the
system declines to build; it is something it has no place to compute.

**Where the constraint lives in code.** `PassportRepository` is the primary
persistence surface — resolver scan telemetry is processed data too — and its
`list` and `count` methods are the only ones that see more than one passport. The port documents the Art. 78(d) limit on
implementors directly, so a future backing store cannot acquire an analytics
sideline without someone editing past the constraint.

**What it does not restrict.** An operator analysing its own passports. The
prohibition binds the processor acting on the operator's behalf, not the
operator.

**Already applied.** Resolver scan telemetry records only per-passport, per-day,
per-variant counts — no IP address, user agent or session identifier, because the
schema has no column for one. That design predates this section; Art. 78(d) is
the article it answers to.

**Residual, host-side.** `dpp-core` is stateless and holds no data, so it can
only state the constraint and place it at the seam. Enforcement — retention of
logs, backup handling, what a hosted control plane may read — is a concern of
whatever hosts it and of that host's infrastructure, and is not evidenced here.

## Personal Data — ESPR Art. 10(1)(e)

ESPR Art. 10(1)(e) makes it an essential requirement that *"personal data
relating to customers shall not be stored in the digital product passport
without their explicit consent in compliance with Article 6 of Regulation (EU)
2016/679"*. Regulation (EU) 2025/2509 (toys) Art. 20(10) and Regulation (EU)
2026/405 (detergents) Art. 22(h) set the same condition for their passports.
Regulation (EU) 2023/1542 has no equivalent, so a battery passport answers to
Regulation (EU) 2016/679 (GDPR) and to its own Art. 78(h) requirement of a high
level of privacy.

**Personal data a passport is not required to carry is kept out of it.**
Consent can be withdrawn at any time (GDPR Art. 7(3)), and withdrawal obliges
erasure where no other legal ground for the processing remains (Art.
17(1)(b)). A published passport is signed, frozen, archived and copied, so
nothing inside it can be erased. Data the governing act requires,
such as a battery's Annex XIII point 4 usage record, is a different case: it
rests on a legal obligation (GDPR Art. 6(1)(c)), and erasure does not reach it
(Art. 17(3)(b)).

**Where it lives in code.** Product-group schemas mark operator-written free
text that describes one item's life after sale with `x-personal-data`.
`dpp_domain::check_personal_data`, run by `validate_passport`, refuses a marked
field with a value unless the passport's `personalData` states what is held
about it. That is either nothing, or an erasable record held outside the
passport through `PersonalDataPort`, on a named GDPR Art. 6(1) basis. Where a
governing act admits customer personal data only with explicit consent, any
other basis is refused. That condition is recorded per act in the instrument
manifests. `redact_passport` shows a statement only to the audiences that see
its field, and never to the public.

**What it does not do.** Read field content. Whether text is personal data
cannot be decided by a pattern, and the statement is the controller's answer,
signed with its own key. Nor can it check that a basis is valid or that consent
was given. Those are the controller's to establish and to demonstrate (GDPR
Art. 5(2), Art. 7(1)).

**When it binds.** Art. 10(1)(e) binds a product group once a delegated act
under ESPR requires its passport, and none does yet. The toys and detergents
passports apply from 1 August 2030 and 23 September 2029. GDPR applies now, to
any personal data a passport or a draft holds, whichever act governs it.

**Residual, host-side.** The check is a function, and a host decides when it
runs. Where one of those acts applies, the prohibition is on *storing*, and a
stored draft is storage, so the check belongs on every write and not only at
publish. Running it on every write today costs nothing and keeps the passport
inside GDPR's minimisation principle before any of them applies. Holding the
records, erasing them on withdrawal, and keeping them out of every served view
are the host's, through the port.

## Cryptographic Foundations

- **Ed25519** — All signing operations use Ed25519 (EdDSA). The curve is a
  design choice of this library: no provision of ESPR or of its implementing
  acts is recorded as requiring it.
- **AES-256-GCM** — Key encryption at rest.
- **did:web** — DID method for operator identification, with DID Document
  builder following W3C DID Core v1.0.
- **JWS (RFC 7515)** — Compact serialisation for passport and transfer signatures.

## Wasm Plugin Architecture

Product group-specific compliance logic runs as sandboxed Wasm modules
(`wasm32-wasip1`) loaded by a host. The plugin ABI includes:

- Capability negotiation (plugins declare supported operations).
- Semantic versioning with compatibility checking.
- A stateless contract: each call carries its whole input, and a plugin is a
  unit struct with no state of its own. Whether a host reuses a Wasm instance
  between calls is that host's decision and is not evidenced here.

## Test Coverage

| Test Suite | Location | Coverage |
|---|---|---|
| Textile end-to-end | `crates/dpp-tests/tests/textile_end_to_end.rs` | Passport lifecycle, AAS, GS1, credentials |
| Transfer of responsibility | `crates/dpp-tests/tests/transfer_of_responsibility.rs` | Transfer chain, provenance, error cases |
| Audience gatekeeping | `crates/dpp-tests/tests/access_gatekeeping.rs` | All three audiences, edge cases, custom policies |
| Schema conformity | `crates/dpp-tests/tests/schema_conformity.rs` | Schema validity, textile field-set regression — not a conformity check |
| Unit tests | Per-module `#[cfg(test)]` | All crates have inline unit tests |

## CI/CD Gate

The `check` recipe in `justfile` is the local gate, and its dependency list is
the list of what it runs — formatting, lints and tests for the workspace and
for the sector plugins, doc-tests, the doc build, the lockfile check and
`cargo audit`. It is not restated step by step here, because the four-step copy
this section used to carry had fallen behind it. GitHub Actions
(`.github/workflows/`) covers the same ground in separate jobs and adds what
`check` cannot: the Wasm cross-compiles, the orphaned-tests guard, and the GS1,
AAS and JAdES oracle suites in their own workflows.

## Known Gaps

1. **DID resolution.** `dpp_crypto::jws` verifies an EdDSA signature
   cryptographically against a public key the caller supplies. Fetching a
   `did:web` document over the network to obtain that key needs an HTTP
   client, which the pure core does not have, so a host does it.

2. **Status list fetching.** Revocation is decided against a W3C Bitstring
   Status List v1.0: `dpp_vc::status_list` decodes the list and
   `check_revocation` reads the credential's bit, failing closed on an index
   the list cannot answer. Fetching the status list credential needs an HTTP
   client, so a host supplies the list.

3. **Schema hot-reload** is implemented but the file-watching trigger lives
   in a host crate.

4. **Wasm plugins** are excluded from the Cargo workspace. `just check` still
   formats, lints and tests them through their own recipes, and the
   `wasm-build.yml` workflow cross-compiles every plugin to `wasm32-wasip1`.

## Contact

For conformity assessment inquiries: dev@odal-node.io
