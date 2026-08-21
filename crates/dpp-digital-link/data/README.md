# Vendored GS1 data

## `gs1-syntax-dictionary.txt`

**Source:** <https://github.com/gs1/gs1-syntax-dictionary>
**Release:** `2026-01-27` (the tagged release, not `main` — `main` carries
`Release: UNSET` and so cannot be attributed to a version)
**Licence:** Apache-2.0, the same licence as this crate. Copyright GS1 AISBL.

### Why this is vendored rather than typed

It defines, for all 224 currently assigned GS1 Application Identifiers, whether
each is **pre-defined length** (the `*` flag — no FNC1 separator required) or
variable length, plus the length of every component.

That distinction is the whole parser. Getting one AI's fixed-vs-variable wrong
silently truncates or over-reads data scanned off a physical product, and the
result still looks like a plausible identifier. It is precisely the class of
detail that must come from the authority rather than from memory, so the table
is derived from this file at runtime and no second copy of it exists in the
source.

### Updating it

Fetch the file at a **tagged** release and check the `Release:` line moved:

```
curl -sSL -o gs1-syntax-dictionary.txt \
  https://raw.githubusercontent.com/gs1/gs1-syntax-dictionary/<tag>/gs1-syntax-dictionary.txt
```

A new release can add AIs, change a length, or reclassify an AI's flags. Treat
the diff as a behaviour change to the parser, because it is one — the tests
that pin specific AI shapes exist to make that visible.

### What it does not give us

Content validation. The dictionary names a *linter* per component (`csum`,
`gcppos1`, …); the reference implementations of those routines live in GS1's
Syntax Tests, which are **not** vendored here. This crate applies the check
digit it already implements and the lengths from this file — it does not claim
to run GS1's full deep validation.
