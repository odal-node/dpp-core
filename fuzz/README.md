# dpp-core fuzz targets

libFuzzer targets for the pure byte-level parsers. Requires **nightly + Linux**
(libFuzzer is not available on the Windows dev box), so these run in the nightly
CI job — not in the stable `just check` gate.

## Targets

| Target | Fuzzes |
|---|---|
| `digital_link_parse` | `DigitalLink::parse` — GS1 Digital Link URI bytes |

## Run locally (Linux)

```sh
cargo install cargo-fuzz
cargo +nightly fuzz run digital_link_parse            # runs until a crash or Ctrl-C
cargo +nightly fuzz run digital_link_parse -- -max_total_time=60   # 60s smoke
```

## Corpus & regressions

Seed the corpus from known-good vectors:

```sh
mkdir -p corpus/digital_link_parse
printf 'https://id.odal-node.io/01/09506000134352/21/ABC123' > corpus/digital_link_parse/seed1
```

When a crash is found, cargo-fuzz writes a reproducer to `artifacts/`. Minimize it
(`cargo +nightly fuzz cmin`, `tmin`) and commit the input as a named regression
**unit test** in the crate it broke — the corpus feeds the ordinary test suite.
