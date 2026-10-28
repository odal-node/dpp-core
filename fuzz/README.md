# dpp-core fuzz targets

libFuzzer targets for the byte-level parsers. They need **nightly Rust on Linux**
(libFuzzer does not support Windows), so they run in the nightly CI job rather than in
`just check`.

## Targets

Each one sits where bytes from outside reach the library. Every target holds the same
property: any input returns `Ok` or `Err` (or `Some` or `None`), never a panic.

| Target | Fuzzes | Where the bytes come from |
|---|---|---|
| `digital_link_parse` | `DigitalLink::parse` on GS1 Digital Link URI bytes | A scanned QR code |
| `plugin_abi_envelope` | `AbiResult` and `PluginCapabilities` JSON | A Wasm plugin's output |
| `jws_verify` | `verify_jws` and `extract_kid_from_jws` on compact tokens, against a key nobody can sign for, so a token that verifies is a failure too | A passport fetched from another operator, a proof file |
| `sd_jwt_parse` | `SdJwt::parse` and `Disclosure::parse` | A holder's selectively disclosed presentation |
| `status_list_decode` | `StatusList::from_encoded_list` (base64url, then gzip) and `get` | A credential issuer's revocation list |
| `passport_from_stored` | `Passport::from_stored`, through the direct shape and the lens chain | A stored or fetched passport document |
| `identifier_parse` | The GTIN, GLN, commodity code, CN category, ICD, web URL, DID and calendar date parsers, and `ProductIdentifier` from JSON | Spreadsheet rows and API bodies |
| `schema_validate` | Any JSON against the newest schema of every product group | A create body |

Every `[[bin]]` in `Cargo.toml` is also listed in the matrix in
`.github/workflows/fuzz.yml`, which is the only place the targets run.
`crates/dpp-tests/tests/fuzz_targets_in_ci.rs` fails when the two lists differ.

## Run locally (Linux)

```bash
cargo install cargo-fuzz
cargo +nightly fuzz run digital_link_parse                          # until a crash or Ctrl-C
cargo +nightly fuzz run digital_link_parse -- -max_total_time=60   # 60-second smoke run
```

## Corpus and regressions

Seed the corpus from known-good vectors:

```bash
mkdir -p corpus/digital_link_parse
printf 'https://id.odal-node.io/01/09506000134352/21/ABC123' > corpus/digital_link_parse/seed1
```

When cargo-fuzz finds a crash, it writes a reproducer to `artifacts/`. Minimise it
(`cargo +nightly fuzz cmin`, `tmin`) and commit the input as a named regression **unit
test** in the crate it broke, so the ordinary test suite keeps checking it.
