# dpp-vocab

[![crates.io](https://img.shields.io/crates/v/dpp-vocab.svg)](https://crates.io/crates/dpp-vocab)
[![docs.rs](https://img.shields.io/docsrs/dpp-vocab)](https://docs.rs/dpp-vocab)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

The register of external vocabularies used by EU Digital Product Passport data: **who
publishes a term, under which licence, and whether anyone here has read it.** Part of
[dpp-core](https://github.com/odal-node/dpp-core) by [Odal Node](https://odal-node.io).

Using another organisation's term tells a machine that our field means what that
organisation says its identifier means. This crate is the single home for that kind of
claim.

## The rule

> An identifier is either in the `urn:odal-node:` namespace, or it is a term someone has
> read, in a vocabulary someone has read. There is no third category.

The two readings are separate. A namespace is a prefix, so every string that begins
with it looks like one of the vocabulary's terms, and reading the vocabulary's
publication does not mean reading each definition in it. Each record therefore lists the
terms read, and only a listed term is permitted.

Declining a claim is better than making one we cannot support. A `urn:odal-node:`
identifier says "this is our concept". A wrong IRDI in its place says "this is ECLASS's
concept", falsely, to a machine, in a format most likely to be consumed without anyone
looking at it.

This has happened. Six identifiers claiming IDTA and ECLASS authority shipped in a
published version of this workspace and were removed once checked: they used a
document-number form the authority does not publish, so no consumer could have resolved
them. A test in this crate keeps all six refused.

## Current state

Six of the twenty records in `vocabularies/` are verified (`batterypass-samm`,
`catena-x-battery-pass`, `ec-battery-guidance`, `gs1`, `idta`, `semic`): for each, a
person read the authority's own publication and recorded what it said. Every other
record is refused. Each record carries the finding behind its status, the step that
would move it on, and pins that show when its source has changed.

Terms read so far: `https://ref.gs1.org/voc/gtin`. A verified vocabulary with an empty
`terms` list permits no term, so reading another GS1 term is a separate task.

```rust
use dpp_vocab::{VocabularyRegister, Verdict};

let register = VocabularyRegister::new();

// Our own concepts need no provenance record.
assert_eq!(
    register.verdict("urn:odal-node:aas:property:co2e-per-unit:1.0"),
    Verdict::Own,
);

// GS1's own definition of `gtin` was read on 2026-08-11 and matches our
// `Gtin` type exactly, so that term is permitted.
assert!(register.verdict("https://ref.gs1.org/voc/gtin").is_permitted());

// Reading one GS1 term does not read the rest of the namespace.
assert!(!register.verdict("https://ref.gs1.org/voc/someOtherTerm").is_permitted());

// Most third-party identifiers are still refused, and the refusal says why.
let verdict = register.verdict("https://schema.org/dateModified");
assert!(!verdict.is_permitted());
println!("{}", verdict.reason());
```

## Layers

Every vocabulary is one of four kinds. Mixing them up is how a data model ends up tied
to someone else's release cycle.

| Layer | Relationship | Example |
|---|---|---|
| `foundational` | Something you **build on** | GS1 Web Vocabulary, EU SEMIC Core Vocabularies |
| `community-profile` | Something you **map to** | UN Transparency Protocol, CIRPASS-2 |
| `sectoral` | A domain vocabulary beside the foundation | GS1 Rail |
| `conformance-bridge` | Something you are **tested by** | CEN/CENELEC JTC 24, Battery Pass SAMM |

A conformance bridge is never used as our vocabulary. Being measured against a model is
not the same as speaking its language, and adopting a bridge's identifiers turns an
external check into an internal dependency.

## Adding a vocabulary

Add one JSON file in `vocabularies/` and one entry in the embedded list. Never put two
authorities in one file: a reader could not tell which one a `finding` belongs to. The
fixed key set and each status are documented in `vocabularies/README.md`.

Using a term from that vocabulary is a second step: read its definition, then list it in
the record's `terms`. A verified record also pins the upstream state it was read at, and
`just vocab-drift` reports when that has moved.

## The mapping layer and its standard

This crate records **vocabularies**. The layer above it records **mappings** ("our term
X corresponds to their term Y"), and that layer does not exist yet.

When it is built it will use [SSSOM](https://arxiv.org/abs/2112.07051) (A Simple
Standard for Sharing Ontological Mappings) rather than a shape of our own. SSSOM requires
four slots per mapping (`subject_id`, `object_id`, `predicate_id` and **`match_type`**)
and recommends drawing `predicate_id` from SKOS or OWL: `skos:exactMatch`,
`skos:closeMatch`, `skos:broadMatch`, `skos:narrowMatch`.

`match_type` matters most here, because it records *how a mapping was derived*, and
SSSOM also says a match should reference the `mapping_tool` and `mapping_tool_version`
that produced it. A mapping asserted by a pipeline and one asserted by a person who read
the specification are different claims, and SSSOM already has a way to say so.

The difference between `exactMatch` and `closeMatch` also matters. `exactMatch` is
transitive (chain three and the result still holds) while `closeMatch` deliberately is
not, so errors do not compound across combined mappings. Borrowing a term because it is
close enough is exactly what `closeMatch` marks, and must never be recorded as
`exactMatch`.

SSSOM also requires identifiers to be CURIEs with a registered prefix, which is what the
`namespaceIri` field in each record supplies.

## What this crate does not do

It records **identity provenance**, not structural conformance. An identifier being
well formed, current and correctly attributed says nothing about whether our field set
matches the model behind it. Nothing here supports the phrases "IDTA-conformant",
"JTC 24 conformant" or any equivalent.

That is because we have not read the standards, **not** because there is nothing to
claim. Whether a presumption of conformity is available at all is a fact about the
Official Journal, and it has one home: the `jtc24` record in `vocabularies/`, with its
own source and `checkedOn` date. Read it there rather than a summary here. An earlier
version of this paragraph said the opposite of what that record now says, and survived
the correction because prose is not tested.

The direction matters. A presumption that *is* available and that we cannot claim is a
gap worth costing; one that does not exist is just a fact. This gap is the first kind.

## Design

A **leaf crate** with no workspace dependencies. Everything else in this workspace
changes when an EU regulation changes. This crate changes when GS1, IDTA or another
authority publishes, on its own schedule, which is why it sits apart from the passport
model.

If this crate ever needs `dpp-domain`, that dependency is the signal that something
passport-shaped has leaked into it.

## Licence

Apache-2.0. The vocabularies *described* here carry their own terms: several are
unstated, at least one is CC BY 4.0 with an attribution obligation, and the JTC 24
standards are sold and may not be redistributed. The records store identifiers and
citations, never an authority's prose.
