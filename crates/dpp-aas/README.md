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
- You want per-product-group submodels rather than a flat key-value dump.
  Typed submodels exist for the product groups whose act is in force; every
  other catalogued group is projected generically, because naming a submodel
  template that no standards body has ratified would be a claim we cannot
  support.

## Example

```rust
use dpp_aas::build_aas_from_passport;
use dpp_domain::{Audience, Passport};

// The projection is always built for an audience: the passport is filtered
// through the disclosure seam before any mapper sees it, so a public shell
// cannot carry a restricted field. There is no unmasked entry point.
fn render_aas(passport: &Passport, gtin: &str) {
    let (shell, submodels) =
        build_aas_from_passport(passport, gtin, Audience::Public).expect("masking");

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

**The rule is enforced by a test, not by a comment.** CI fails on any
identifier that is neither in our own namespace nor a verified vocabulary.

The record is the [`dpp-vocab`](../dpp-vocab) crate, not a file in this one —
one home for this class of claim, regardless of which projection carries it.
`dpp_vocab::is_own` answers the namespace question; `VocabularyRegister`
answers everything else. Identifiers evaluated and not adopted are recorded
there too, under the authority that publishes them (`idta`, `eclass`,
`catena-x-battery-pass`), each with the finding and what would move it on.
**No vocabulary is verified yet, so this crate emits none.**

Adopting a third-party identifier means a person reading the authority's own
source and the vocabulary record moving to `verified` — never editing a status
field on faith.

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-domain` | Provides `Passport`, `SectorData` and the sector catalog — required by this crate |
| `dpp-digital-link` | Supplies the GTIN that becomes `globalAssetId`; no dependency in either direction |

## Minimum Rust version

1.96 (MSRV is enforced in CI)

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
