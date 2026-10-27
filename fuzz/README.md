# dpp-core fuzz targets

libFuzzer targets for the byte-level parsers. They need **nightly Rust on Linux**
(libFuzzer does not support Windows), so they run in the nightly CI job rather than in
`just check`.

## Targets

| Target | Fuzzes |
|---|---|
| `digital_link_parse` | `DigitalLink::parse` on GS1 Digital Link URI bytes |

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
