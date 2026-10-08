# Vendored JSON Schema Draft-07 artifacts

Nothing in this directory is ours. Each file is a verbatim copy of an upstream
file, vendored so CI is hermetic: upstream remains authoritative, and these
copies exist only so the test suite does not depend on network access or on a
URL staying up. They are the evidence behind the JSON Schema row of
`docs/architecture/STANDARDS.md`, and are read by
`crates/dpp-tests/tests/json_schema_draft07.rs`.

## What is pinned

**The meta-schema**, `draft-07-schema.json`.

- Source: `https://json-schema.org/draft-07/schema`, whose `$id` is
  `http://json-schema.org/draft-07/schema#`.
- Retrieved: 2026-10-06.
- Licence: the file states none, and the upstream repository's licence metadata
  does not name one. It is a test fixture in a crate that is not published.

**The test suite**, `test-suite/`.

- Source: `https://github.com/json-schema-org/JSON-Schema-Test-Suite`, files
  under `tests/draft7/`, plus the repository's `LICENSE`.
- Commit: `5b0ee1613e45fcc2bddac00e07c19cd49b00d8a8`.
- Retrieved: 2026-10-06.
- Licence: MIT, in `test-suite/LICENSE`.

| File | Size (bytes) | SHA-256 |
|---|---|---|
| `draft-07-schema.json` | 4979 | `692e1d165e47afcb5f11b2ce1c639635ffa834035d6ecb6bcf3087481dae8404` |
| `test-suite/LICENSE` | 1057 | `837402bd25fad9b704265801ca3f92566a98157c1f9a7acd6f446299ba1c305a` |
| `test-suite/draft7/additionalProperties.json` | 4426 | `6ab2e5091d2d75fc047767b42cafdb493b67f2559b05bf33202a154363824b4c` |
| `test-suite/draft7/const.json` | 10878 | `65d2b152fbbbdd3291beb3dcc048dea1644acf3c59c68b5f72f521b5a927fcf9` |
| `test-suite/draft7/default.json` | 2231 | `b0c16241a36d86af33b33183da33b15f5d883a141fbc2ee451d57ec679f17e7e` |
| `test-suite/draft7/definitions.json` | 719 | `b1159d8adeb80362bfc09b925b20f4bacdf698f8fe437d9859b4403dfff87b24` |
| `test-suite/draft7/enum.json` | 9457 | `069890108f4f83e62c9be9f42af63b5de911555e6644d83cd89c08ff644a43bb` |
| `test-suite/draft7/exclusiveMinimum.json` | 775 | `08435add3250275173074d2b7ad519806baae83be95d7397d3a514b8ad7561d3` |
| `test-suite/draft7/format.json` | 18067 | `8c670c5bbf816d1bb6a7be7db526d31989e2fe3d2f95198ee7c28c34136c8f22` |
| `test-suite/draft7/items.json` | 8076 | `6480f868b34c832265c9ee42a7643e16873c74f11c8d735d8eea5e494c93ed02` |
| `test-suite/draft7/maxLength.json` | 1297 | `3330342d36c85a7334ea3f7deb168beae26617ce83cc1d43b6ba2f618b3c2f4e` |
| `test-suite/draft7/maximum.json` | 1474 | `b3094bf70bea8df1eb17c22a9afb47af1bcf3b8d6616bfb8bce95c1ee874e2e3` |
| `test-suite/draft7/minItems.json` | 1116 | `706f786718b5af39ca447d006909aa3adba53ebffe450d97cc7773f47d97bb46` |
| `test-suite/draft7/minLength.json` | 1287 | `6faf7b73d1eac9615da33147a05e0fdde90737b62a175d1ce2a9a0bc98eaaaa5` |
| `test-suite/draft7/minimum.json` | 1930 | `8e13e2a245ecffc0c39481430583ef3b11ce1ac3eb5d02617bfe6f8a022b1579` |
| `test-suite/draft7/oneOf.json` | 7254 | `0eff07bc5ec108eb324aeb61d9ff130a3e689b0ab4b26af40561e6b1eda3a26c` |
| `test-suite/draft7/optional/format/date-time.json` | 7997 | `704a361f215c08de56c9aac3ba9b1db674ba7ad097fe286b4bbe5ed8f19a79b6` |
| `test-suite/draft7/optional/format/date.json` | 15066 | `2f3f1ef1b2abc108985bbe286bf334da74b58af74181414169b82e9dfc869271` |
| `test-suite/draft7/optional/format/uri.json` | 8802 | `b35144c96aa5042d1464b14cabfdff5cf7d761bb34b84eb9b07ec54a68ffe1f9` |
| `test-suite/draft7/pattern.json` | 1539 | `92f42bdbf24f20b2b6378a8dfa882a8b40746170303f5dbe5b4cade420599507` |
| `test-suite/draft7/properties.json` | 7305 | `b8246287c2b214d0e17ed4b1904b92cfde803d52546a901ce123a7ecce5b75cb` |
| `test-suite/draft7/ref.json` | 30314 | `217aebe267fab77ff9bb3332d559054cf1602077abb3395dae417c0b6db6d789` |
| `test-suite/draft7/required.json` | 4527 | `66946289772a5e931835060187b18e63bc215fbebf981b2d40d1c945145676f4` |
| `test-suite/draft7/type.json` | 13408 | `091aa31e688df20891de7884878b527745ddac0a3ced6d19a5ea4aa075dbbe00` |

The sizes and hashes are checked by `the_vendored_files_are_the_bytes_recorded_here`,
which also fails on a file in this directory that the table does not list. All
of them are pinned `-text` in `.gitattributes`: line-ending translation would
change the bytes on disk without changing the blob, and the recorded hash would
then fail to verify on any checkout that normalises, which is the default on
Windows.

## Why these files and no others

The suite has a file per keyword. Only the files for keywords that a shipped
schema uses are vendored, plus the format files for the formats it declares. The
test computes that set from the shipped schemas and fails when a schema starts
using a keyword or format whose file is missing, and the message names the file
to take from the commit above. A new file goes in this table in the same change.

The remote-reference file, `refRemote.json`, is not vendored: the workspace
builds `jsonschema` without remote retrieval, and a test checks that no shipped
schema has a `$ref` that leaves its own document.

Every vendored case passes, and none is excluded.
