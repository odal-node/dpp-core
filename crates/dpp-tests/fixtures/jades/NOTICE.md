# Vendored JAdES JSON Schemas

Nothing in this directory is ours. Each file is a verbatim copy of a file ETSI
publishes for ETSI TS 119 182-1, vendored so CI is hermetic: upstream remains
authoritative, and these copies exist only so the test suite does not depend on
network access or on a URL staying up. They are the evidence behind the JAdES row
of `docs/architecture/STANDARDS.md`, and are read by
`crates/dpp-tests/tests/jades_annex_b_schemas.rs`.

## What is pinned

Annex B of ETSI TS 119 182-1 V1.2.1 (2024-07) is normative, and names these
files. They are the ones at tag `v1.2.1`, the revision that annex names, of
`https://forge.etsi.org/rep/esi/x19_182_JAdES`, under `raw/v1.2.1/`.

- Retrieved: 2026-10-06.
- JSON Schema dialect: Draft-07, which `docs/architecture/STANDARDS.md` records
  as the dialect this repository checks and runs.
- Licence: BSD-3-Clause, copyright ETSI, in `ts-119-182-1-v1.2.1/LICENSE`.

| File | Size (bytes) | SHA-256 |
|---|---|---|
| `ts-119-182-1-v1.2.1/19182-jsonSchema.json` | 10898 | `a8241b41ce3448cb0f8b2a235779dd039efc710b0aa721c965c828fcc5c61d6a` |
| `ts-119-182-1-v1.2.1/19182-protected-jsonSchema.json` | 1778 | `9a5c063fbb51e7cab25d2bdd64f90d9fdc8a79a4facdf769cc2f20a72a25cb98` |
| `ts-119-182-1-v1.2.1/19182-unprotected-jsonSchema.json` | 330 | `c9c50655ff101de50ae743f52fa0923a8ce13ef4679da4b0256426d4ab59b11b` |
| `ts-119-182-1-v1.2.1/LICENSE` | 1476 | `a2138586a9114057d86342dee602154c3e7ab050cd7725d4f7ea2620888223d4` |
| `ts-119-182-1-v1.2.1/rfcs/rfc7515-jws.json` | 103 | `25f4de9ac183a3360a5e0e3b1382088d9c1b3eb21a0ba3468aca318c5bdfb016` |
| `ts-119-182-1-v1.2.1/rfcs/rfc7515-protected.json` | 118 | `895ce4319d202f2bd7ac1d1181bb201e7310a83f250354ab47d1c76848dd600f` |
| `ts-119-182-1-v1.2.1/rfcs/rfc7515-unprotected.json` | 120 | `164372b9b3d48e12f56aca47e26ac1642103d4c27545aeaf779f67feb992dae1` |
| `ts-119-182-1-v1.2.1/rfcs/rfc7515.json` | 2733 | `084e7ac7ff284101a282312027036946531dede71ea723d49936aae43a802060` |
| `ts-119-182-1-v1.2.1/rfcs/rfc7517.json` | 1065 | `02799ec31e4383eb39c93b86070e48022e665ac5996c14500a3da913a08c522c` |
| `ts-119-182-1-v1.2.1/rfcs/rfc7797.json` | 127 | `40e72c46f36bcc16ddca4f1bf4bdb6c0012e562bf84883418b14b57c74e25b36` |

The sizes and hashes are checked by `the_vendored_files_are_the_bytes_recorded_here`,
which also fails on a file in this directory that the table does not list. All of
them are pinned `-text` in `.gitattributes`: line-ending translation would change
the bytes on disk without changing the blob, and the recorded hash would then fail
to verify on any checkout that normalises, which is the default on Windows.

## How they are used

The protected-header schema refers to the other files by relative path, such as
`rfcs/rfc7515.json#/definitions/jwsProtectedHeader`. The workspace builds
`jsonschema` without any way to fetch, so the test serves these copies from
memory, as though they were at the location above. Nothing is fetched from it.

Only the protected-header schema is applied to what this repository builds. The
unprotected-header schema, which is for the `etsiU` container of the higher
levels, and the JWS-level files, are vendored because the protected-header schema
reaches them or because the annex names them as one set.
