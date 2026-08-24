# Vendored AAS metamodel JSON Schemas

`aas-3.0.json`, `aas-3.1.json` and `aas-3.2.json` are not our files. Each is a
verbatim copy of IDTA's published JSON Schema for the Asset Administration Shell
metamodel, at the revision named below, vendored so CI is hermetic — upstream
remains authoritative, and these copies exist only so the test suite does not
depend on network access or on a URL staying up.

## What is pinned

Common to all three: source `https://github.com/admin-shell-io/aas-specs`, path
`schemas/json/aas.json`, JSON Schema dialect draft 2019-09, licence CC BY 4.0
(per the upstream repository's licence metadata).

| | `aas-3.0.json` | `aas-3.1.json` | `aas-3.2.json` |
|---|---|---|---|
| **Metamodel version** | 3.0 | 3.1 | 3.2 |
| **Schema title** | `IDTA-01001-3-0-1 AAS JSON Schema` | `IDTA-01001-3-1 AAS JSON Schema` | `IDTA-01001-3-2 AAS JSON Schema` |
| **Upstream tag** | `IDTA-01001-3-0-1_schemasV3.0.9` | `v3.1.2` | `v3.2.0` |
| **Tag commit date** | 2024-11-15 | 2025-10-28 | 2026-07-20 |
| **Retrieved** | 2026-08-02 | 2026-08-04 | 2026-08-04 |
| **Size (bytes)** | 40 005 | 52 421 | 56 193 |
| **SHA-256** | `eb566d47316c99093805bf03545ca4d5439ec4568c218742b7640e39b1548349` | `9a8a005e303634f5041b9a2eac2e19c5cca05b9dd54c4bd20afbc7ab809d411e` | `ffc64ab21d812d2c80a6065c4c87ab3731dfe4144d75a8d4aa2679752358c21f` |

All three are pinned `-text` in `.gitattributes`: line-ending translation would
change the bytes on disk without changing the blob, and the recorded hash would
then fail to verify on any checkout that normalises — the default on Windows.

## Why all three, rather than a chosen one

Every `Environment` this crate builds is validated against **all three
revisions**, and must satisfy every one of them.

The earlier version of this document pinned 3.0 alone and justified it by
claiming 3.0 was "the strictest of the three for our subset". **That was wrong**,
and the test suite now says so out loud. Neither revision is stricter; the
`idShort` name rule moved in both directions:

| | 3.0 | 3.1 / 3.2 |
|---|---|---|
| pattern | `^[a-zA-Z][a-zA-Z0-9_]*$` | `^[a-zA-Z][a-zA-Z0-9_-]*[a-zA-Z0-9_]+$` |
| `state-of-health-pct` | rejected | accepted |
| `a` | accepted | rejected |

3.1 permits interior hyphens that 3.0 forbids, and its trailing `+` requires two
or more characters where 3.0 accepts one. Validating against either alone leaves
the other's rule unenforced.

That matters here more than it would elsewhere. `idShort` is the one constraint
whose satisfaction depends on a passport's *contents* rather than on our code:
the generic product group mapper builds names from operator-supplied JSON keys. A
one-character key produces a one-character `idShort`, which 3.0 accepts and 3.1
tooling rejects. Pinning 3.0 alone would have shipped that gap.

So the target is the **intersection**, which is what "an integrator's AAS
toolchain, whichever revision it implements" actually means in practice. The
divergence is asserted by `the_idshort_rule_diverges_between_revisions` rather
than merely described here, so if a future revision converges the two rules, a
test says so instead of this document going quietly stale.

3.0 is retained rather than dropped in favour of the newest: it is the revision
most deployed tooling implements, and it is the only one of the three that
enforces the stricter no-hyphen name rule.

## What passing this proves, and what it does not

Validating against these files establishes **metamodel validity** — the document
is shaped the way the AAS metamodel says a document is shaped.

It is **not** a conformance claim. IDTA conformance would need IDTA's own test
engine run against a pinned version, and even that would say nothing about
whether a submodel matches a published *submodel template*. Any public wording
about this must say "schema-valid against IDTA-01001 metamodel 3.0, 3.1 and
3.2", never "IDTA-conformant".

## The external loader, and what it covers

A separate CI job (`.github/workflows/aas-oracle.yml`) runs every committed
Environment through an external AAS implementation, because the blind spot below
means no schema can do this job.

| | |
|---|---|
| **Tool** | `aas-core3.0` (aas-core-works, Python) |
| **Pinned at** | `1.1.4` |
| **Metamodel it implements** | AAS **3.0** |
| **Checks** | deserialisation, the specification's own constraint verification, and round-trip |

**It covers metamodel 3.0 only.** We validate documents against the 3.0/3.1/3.2
schemas but load them through a 3.0 implementation, so the loader-level check —
the one that catches members no schema sees — is 3.0-scoped. Do not round that
claim up. `aas-core3.0` is generated from `aas-core-meta`, so a 3.1/3.2 loader
becomes available if and when upstream generates one.

The pin is exact and deliberate: this is somebody else's judgement about what the
metamodel permits, and an unpinned upgrade would turn their tightening into a red
build on code that did not change.

**A lenient implementation is worth nothing here.** Eclipse BaSyx accepts unknown
members and reported success on the `unit` defect that made every Environment
unloadable. An oracle has to be strict to be an oracle — which is also why
"opens in a GUI tool" is not evidence.

Public wording: "passes `aas-core3.0` 1.1.4 for metamodel 3.0". Never
"IDTA-conformant" — that is a separate process against IDTA's own test tooling,
and neither claim says anything about submodel-template conformance.

### The blind spot, named because it caught us

**None of these schemas sets `additionalProperties` anywhere.** A member that is
not part of a class therefore validates in silence, in every revision. We emitted
`kind` on `AssetAdministrationShell` — a member of `HasKind`, which `Submodel`
composes and the shell does not — for several releases, and no schema gate could
ever have reported it. What rejects such a document is a strict AAS loader, not a
validator.

`the_shell_carries_no_member_outside_the_metamodel` is the separate gate for
that class of defect. It checks emitted members against the class's own member
set, derived from these files by following the `allOf`/`$ref` inheritance chain.
Upgrading revisions would not have helped: 3.0, 3.1 and 3.2 all accept the extra
member equally.

## Updating

Re-download from the tag, record the new tag, commit date, retrieval date, size
and hash above, and state why the revision moved. Update the per-revision
UTF-16 pattern count in `VENDORED_SCHEMAS` (`tests/all_product_groups_aas.rs`) — the
suite asserts each count exactly, so a changed schema fails loudly rather than
dropping more constraints than intended.

Adding a revision means adding a row here, a file, and an entry in
`VENDORED_SCHEMAS`; the `.gitattributes` pin is a glob and needs no change.

Do not edit these files — a locally modified copy of someone else's schema is no
longer evidence of anything.
