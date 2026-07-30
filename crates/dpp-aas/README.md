# dpp-aas

[![crates.io](https://img.shields.io/crates/v/dpp-aas.svg)](https://crates.io/crates/dpp-aas)
[![docs.rs](https://img.shields.io/docsrs/dpp-aas)](https://docs.rs/dpp-aas)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

[Asset Administration Shell](https://industrialdigitaltwin.org/en/content-hub/aasspecifications)
(IDTA) projection of an [Odal Node](https://odal-node.io) Digital Product
Passport — shells and submodels for Industry 4.0 and Catena-X data spaces.

Pure Rust, no I/O, no network calls. `wasm32-unknown-unknown` safe.

⚠️ **This crate emits AAS-shaped output; it does not claim IDTA conformance.**
Producing a valid-looking Environment is not the same as conforming to a
specification, and nothing here should be described as IDTA-conformant.

## When to use this crate

- You need to render a typed `Passport` as an AAS shell plus submodels for an
  industrial data space.
- You are submitting to a registry that speaks AAS rather than raw DPP JSON.
- You want per-product-group submodels (battery, textile, electronics and eight
  more) rather than a flat key-value dump.

## Example

```rust
use dpp_aas::build_aas_from_passport;
use dpp_domain::Passport;

fn render_aas(passport: &Passport, gtin: &str) {
    let (shell, submodels) = build_aas_from_passport(passport, gtin);

    assert_eq!(shell.id_short, "DigitalProductPassport");
    assert!(shell.asset_information.global_asset_id.contains(gtin));

    // Five core submodels, plus one per-sector submodel when sector_data is set
    for submodel in &submodels {
        println!(
            "{} ({} elements)",
            submodel.id_short,
            submodel.submodel_elements.len()
        );
    }
}
```

A runnable version is in [`examples/passport_to_aas.rs`](examples/passport_to_aas.rs).

## Semantic identifiers

A `semanticId` tells an integrator's toolchain that one of our elements is
semantically identical to a published concept. It is a claim about another
organisation's vocabulary, made in the format most likely to be consumed
without a human ever reading it — so this crate holds one rule:

> Every emitted `semanticId` is either in the `urn:odal-node:` namespace, or it
> carries a provenance record naming who verified it against the authority's own
> published source, and when.

**The rule is enforced by a test, not by a comment.** An entry missing
`verifiedOn` or `verifiedBy` is refused, and CI fails on any identifier that
satisfies neither branch.

The record is [`src/semantic_ids/allowlist.json`](src/semantic_ids/allowlist.json).
It has two sections with fixed key sets, also test-enforced:

- **`allowlist`** — identifiers this crate may emit. Currently empty: **it emits
  none.** The claim is therefore the narrow, accurate one — AAS-shaped output
  carrying our own semantics, not IDTA conformance.
- **`tracked`** — identifiers evaluated and not adopted, each with the reason,
  the correct value where established, and what to check before adopting it.
  Nothing here is permitted; a test asserts it.

Adopting a third-party identifier means moving it between those sections and
naming a reader — never editing a status field.

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-domain` | Provides `Passport`, `SectorData` and the sector catalog — required by this crate |
| `dpp-digital-link` | Supplies the GTIN that becomes `globalAssetId`; no dependency in either direction |

## Minimum Rust version

1.96 (MSRV is enforced in CI)

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
