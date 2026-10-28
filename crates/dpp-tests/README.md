# dpp-tests

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

Cross-crate integration tests and structural checks for `dpp-core`. Not published to
crates.io (`publish = false`). As a workspace member it runs with every
`cargo test --workspace` and in CI.

The tests in `tests/` cover two things:

- **Behaviour across crates:** credential issuance, passport generation, transfer of
  responsibility, audience gatekeeping, Digital Link handling, AAS mapping, schema
  conformance.
- **Drift and layout checks:** the crate graph in `CONTRIBUTING.md`, the schema table and
  headings in `DATA-MODEL.md`, the port inventory in `PORTS.md`, the standards register,
  the publication boundary, file layout rules, and the generated `SCHEMA-CHANGES.md`.

## Run the tests

```bash
cargo test -p dpp-tests
```

Or the whole workspace:

```bash
cargo test --workspace
```

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-domain` | Domain objects used by the tests |
| `dpp-crypto` | Crypto primitives used by the tests |
| `dpp-vc` | Credentials and access gatekeeping |
| `dpp-digital-link` | GS1 Digital Link tests (dev dependency) |
| `dpp-aas` | AAS submodel mapping tests (dev dependency) |
| `dpp-registry`, `dpp-vocab`, `dpp-rules` | Registry, vocabulary and rule tests (dev dependencies) |

## Minimum Rust version

1.96, enforced in CI.

## Licence

Apache-2.0. See [LICENSE](../../LICENSE).
