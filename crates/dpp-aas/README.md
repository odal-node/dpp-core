# dpp-aas

[![crates.io](https://img.shields.io/crates/v/dpp-aas.svg)](https://crates.io/crates/dpp-aas)
[![docs.rs](https://img.shields.io/docsrs/dpp-aas)](https://docs.rs/dpp-aas)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

[Asset Administration Shell](https://industrialdigitaltwin.org/en/content-hub/aasspecifications)
(IDTA) projection of an EU Digital Product Passport: shells and submodels for Industry
4.0 and Catena-X data spaces. Part of [dpp-core](https://github.com/odal-node/dpp-core)
by [Odal Node](https://odal-node.io).

Pure Rust, no I/O, no network calls. Builds for `wasm32-unknown-unknown`.

> **This crate emits AAS-shaped output; it does not claim IDTA conformance.** A
> valid-looking Environment is not conformance to the specification, and nothing here
> should be described as IDTA-conformant.

## When to use this crate

- You need to render a typed `Passport` as an AAS shell plus submodels for an
  industrial data space.
- You submit to a registry that speaks AAS rather than raw DPP JSON.
- You want per-product-group submodels rather than a flat key-value dump. Typed
  submodels exist for `battery`, `electronics` and `unsold_goods`, whose acts are in
  force, and for `textile` as a named exception. Every other catalogued group is projected
  generically, because naming a submodel template that no standards body has ratified
  would be an unsupported claim.

## Example

```rust
use dpp_aas::{AssetIdentity, build_aas_from_passport};
use dpp_domain::{Audience, Passport};

// The projection is always built for an audience: the passport is filtered
// through the disclosure seam before any mapper sees it, so a public shell
// cannot carry a restricted field. There is no unmasked entry point.
fn render_aas(passport: &Passport) {
    // What the shell says the asset *is*. `from_passport` uses the passport's
    // own EN 18219 identifier and names the `specificAssetId` after the scheme
    // that issued it, so the label cannot disagree with the value.
    //
    // `None` is a real answer, not a failure: an unsold-goods report identifies
    // a reporting period rather than a trade item. Such a caller supplies
    // `AssetIdentity::Named { name, value }` instead, and may not reuse a
    // scheme's name for it.
    let identity = AssetIdentity::from_passport(passport).unwrap_or(AssetIdentity::Named {
        name: "assetKey",
        value: "urn:example:asset:1",
    });

    let (shell, submodels) =
        build_aas_from_passport(passport, identity, Audience::Public).expect("masking");

    assert_eq!(shell.id_short, "DigitalProductPassport");

    // Five core submodels, plus one per-product group submodel when product_group_data is set
    for submodel in &submodels {
        println!(
            "{} ({} elements)",
            submodel.id_short,
            submodel.submodel_elements.len()
        );
    }
}

// The rule the shell follows, asserted against literals rather than against
// the builder's own helper. `global_asset_id.contains(identity.value())` would
// have passed just as happily under the double-wrapping this replaced, because
// the wrapped form contains the value too. An assertion that recomputes the
// expected value the way the code does cannot catch the code being wrong.
//
// A scheme 2 link and a scheme 3 DID are identifiers already; only a bare GTIN
// is not, so only a bare GTIN is wrapped.
fn global_asset_ids_are_not_double_wrapped(passport: &Passport) {
    use dpp_domain::identifier::ProductIdentifier;

    let did = ProductIdentifier::did("did:web:example.com:item:1").expect("a valid DID");
    let (shell, _) =
        build_aas_from_passport(passport, AssetIdentity::Product(&did), Audience::Public)
            .expect("masking");
    assert_eq!(
        shell.asset_information.global_asset_id,
        "did:web:example.com:item:1"
    );

    let gtin = ProductIdentifier::gs1(dpp_domain::Gtin::parse("09506000134352").expect("a GTIN"));
    let (shell, _) =
        build_aas_from_passport(passport, AssetIdentity::Product(&gtin), Audience::Public)
            .expect("masking");
    assert_eq!(
        shell.asset_information.global_asset_id,
        "urn:odal-node:product:09506000134352"
    );
}
```

A runnable version is in [`examples/passport_to_aas.rs`](examples/passport_to_aas.rs).

## Semantic identifiers

A `semanticId` tells an integrator's toolchain that an element is semantically
identical to a published concept. It is a claim about another organisation's
vocabulary, in a format most likely to be consumed without a person reading it. So
this crate follows one rule:

> Every emitted `semanticId` is either in the `urn:odal-node:` namespace, or carries a
> provenance record naming who verified it against the authority's own published
> source, and when.

**A test enforces the rule.** CI fails on any identifier that is neither in our own
namespace nor a verified vocabulary.

The record lives in the [`dpp-vocab`](../dpp-vocab) crate rather than here, so this
class of claim has one home whichever projection carries it. `dpp_vocab::is_own`
answers the namespace question and `VocabularyRegister` answers the rest. Identifiers
evaluated and not adopted are recorded there too, under the authority that publishes
them (`idta`, `eclass`, `catena-x-battery-pass`), each with the finding and what would
change it. **No vocabulary is verified yet, so this crate emits none.**

Adopting a third-party identifier means a person reads the authority's own source and
the vocabulary record moves to `verified`. A status field is never edited on faith.

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-domain` | Provides `Passport`, `ProductGroupData` and the product group catalog; required |
| `dpp-digital-link` | Supplies the GTIN that becomes `globalAssetId`; no dependency in either direction |

## Minimum Rust version

1.96, enforced in CI.

## Licence

Apache-2.0. See [LICENSE](../../LICENSE).
