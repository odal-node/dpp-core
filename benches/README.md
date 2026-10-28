# dpp-benches

Criterion micro-benchmarks for the `dpp-core` operations that run on every passport
publish, export or verification. Not published (`publish = false`) and not part of the
public API.

## Run

```bash
just bench                          # all benchmarks (cargo bench --package dpp-benches)
cargo bench --package dpp-benches   # the same
cargo bench -p dpp-benches crypto   # one bench target
```

Criterion writes HTML reports and regression baselines to `target/criterion/`. Each run
compares against the previous baseline, so you can see whether a change moved a number.

## Targets

| Target | Source | Measures |
|---|---|---|
| `crypto` | [src/crypto.rs](src/crypto.rs) | Ed25519 and JWS: signing a payload, verifying through the keystore, verifying against a raw public key |
| `validation` | [src/validation.rs](src/validation.rs) | JSON Schema validation: one battery, one textile, and a 100-record mixed batch. Validators are warmed first, so schema compilation is not measured |
| `gs1` | [src/gs1.rs](src/gs1.rs) | GS1 Digital Link parsing (AI 01/21 and full 01/10/21 URIs) |
| `calc` | [src/calc.rs](src/calc.rs) | Cradle-to-gate CO2e (small and 50-material bills) and the simplified repairability heuristic |
| `aas` | [src/aas.rs](src/aas.rs) | AAS submodel mapping (`build_aas_from_passport`) and the build-and-serialise path used for Catena-X and IDTA export |

## Conventions

- Each target is a standalone Criterion binary (`harness = false`) registered as a
  `[[bench]]` in [Cargo.toml](Cargo.toml).
- Setup (key generation, validator warm-up, fixtures) happens **outside** the
  `b.iter(...)` closure, so only the operation under test is timed.
- Fixtures use realistic product group data (a battery passport with Annex XIII fields,
  a 60/40 cotton and polyester textile), so the numbers reflect production-shaped
  inputs.

When you add a hot path to `dpp-core`, add a bench target: a new `src/<name>.rs` and a
`[[bench]]` entry in [Cargo.toml](Cargo.toml).
