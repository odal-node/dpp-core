# Vendored AAS metamodel JSON Schema

`aas.json` is not our file. It is a verbatim copy of IDTA's published JSON
Schema for the Asset Administration Shell metamodel, vendored so CI is hermetic
— upstream remains authoritative, and this copy exists only so the test suite
does not depend on network access or on a URL staying up.

| | |
|---|---|
| **Schema title** | `IDTA-01001-3-0-1 AAS JSON Schema` |
| **Schema `$id`** | `https://admin-shell.io/aas/3/0` |
| **Metamodel version** | AAS **3.0** (specification IDTA-01001-3-0-1) |
| **Source** | `https://github.com/admin-shell-io/aas-specs` |
| **Path** | `schemas/json/aas.json` |
| **Tag** | `IDTA-01001-3-0-1_schemasV3.0.9` |
| **Retrieved** | 2026-08-02 |
| **Size** | 40 005 bytes |
| **SHA-256** | `eb566d47316c99093805bf03545ca4d5439ec4568c218742b7640e39b1548349` |
| **Licence** | CC BY 4.0 (per the upstream repository's licence metadata) |
| **JSON Schema dialect** | draft 2019-09 |

## Why 3.0 and not 3.1 or 3.2

Upstream's default branch carries `IDTA-01001-3-2`, and tags exist for the 3.1
line. We validate against **3.0** deliberately:

- 3.0 (April 2023) is the revision deployed tooling actually implements. The
  audience for this door is an integrator's existing AAS toolchain, and a
  document that satisfies the version their tooling was built against is worth
  more than one that satisfies the newest published revision.
- Our Environment uses a small, stable corner of the metamodel — shells,
  submodels, properties, collections, reference elements. A document valid
  against 3.0 here is very likely valid against 3.1 and 3.2; the converse is
  not guaranteed, so 3.0 is the stricter target of the three for our subset.

Pinning is the point: the vendored copy is what makes the answer permanent, so
moving to another revision is a deliberate change with a visible diff here, not
something that happens because upstream's branch moved.

## What passing this proves, and what it does not

Validating against this file establishes **metamodel validity** — the document
is shaped the way the AAS metamodel says a document is shaped.

It is **not** a conformance claim. IDTA conformance would need IDTA's own test
engine run against a pinned version, and even that would say nothing about
whether a submodel matches a published *submodel template*. Any public wording
about this must say "schema-valid against IDTA-01001-3-0-1", never
"IDTA-conformant".

## Updating

Re-download from the tag, record the new tag, date, size and hash above, and
state why the revision moved. Do not edit `aas.json` — a locally modified copy
of someone else's schema is no longer evidence of anything.
