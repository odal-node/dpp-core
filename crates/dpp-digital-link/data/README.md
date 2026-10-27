# Vendored GS1 data

## `gs1-syntax-dictionary.txt`

**Source:** <https://github.com/gs1/gs1-syntax-dictionary>
**Release:** `2026-01-27`, the tagged release. `main` carries `Release: UNSET` and cannot
be attributed to a version.
**Licence:** Apache-2.0, the same as this crate. Copyright GS1 AISBL.

### Why it is vendored rather than typed in

It defines, for all 224 currently assigned GS1 Application Identifiers, whether each is
**pre-defined length** (the `*` flag: no FNC1 separator needed) or variable length, and
the length of every component.

That distinction is the parser. Getting one AI's fixed or variable length wrong silently
truncates or over-reads data scanned from a physical product, and the result still looks
like a plausible identifier. This detail must come from the authority, not from memory,
so the table is derived from this file at runtime and no second copy exists in the
source.

### Updating it

Fetch the file at a **tagged** release and check that the `Release:` line moved:

```bash
curl -sSL -o gs1-syntax-dictionary.txt \
  https://raw.githubusercontent.com/gs1/gs1-syntax-dictionary/<tag>/gs1-syntax-dictionary.txt
```

A new release can add AIs, change a length or reclassify an AI's flags. Treat the diff as
a behaviour change to the parser, because it is one. The tests that pin specific AI
shapes exist to make that visible.

### What it does not give us

Content validation. The dictionary names a *linter* for each component (`csum`,
`gcppos1` and others), and the reference implementations of those routines live in GS1's
Syntax Tests, which are **not** vendored here. This crate applies the lengths and
character sets from this file and five of the linters it names: `csum`, `csumalpha`,
`zero`, `gcppos1` and `gcppos2`, the last two only as far as the four leading digits of a
GS1 Company Prefix. It does not claim to run GS1's full validation.
