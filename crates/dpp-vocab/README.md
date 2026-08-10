# dpp-vocab

[![crates.io](https://img.shields.io/crates/v/dpp-vocab.svg)](https://crates.io/crates/dpp-vocab)
[![docs.rs](https://img.shields.io/docsrs/dpp-vocab)](https://docs.rs/dpp-vocab)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

The upstream vocabulary register for the [Odal Node](https://odal-node.io)
Digital Product Passport system: **who publishes a term, under what licence, and
whether anybody here has actually read it.**

Naming another organisation's term asserts, to a machine, that our field means
what that organisation says its identifier means. This crate is the one home for
that class of claim.

## The rule

> An identifier is either in the `urn:odal-node:` namespace, or it belongs to a
> vocabulary somebody has read. There is deliberately no third category.

Declining to make a claim beats making one we cannot support. An honest
`urn:odal-node:` identifier says *"this is our concept"*. A wrong IRDI in its
place says *"this is ECLASS's concept"* — falsely, to a machine, in the format
most likely to be consumed without a human ever looking at it.

This rule is not theoretical. Six identifiers claiming IDTA and ECLASS authority
shipped in a published version of this workspace and were removed once checked:
they used a document-number form the authority does not publish, so no consumer
would ever have resolved them. A test in this crate keeps all six refused.

## Current state

**No vocabulary is verified, so every third-party identifier is refused.** That
is the accurate state of this project's knowledge, not a gap filled by default.
Each record in `vocabularies/` carries the finding that got it to its status and
the step that would move it on.

```rust
use dpp_vocab::{VocabularyRegister, Verdict};

let register = VocabularyRegister::new();

// Our own concepts need no provenance record.
assert_eq!(
    register.verdict("urn:odal-node:aas:property:co2e-per-unit:1.0"),
    Verdict::Own,
);

// Nothing third-party is permitted yet — and the refusal says why.
let verdict = register.verdict("https://ref.gs1.org/voc/gtin");
assert!(!verdict.is_permitted());
println!("{}", verdict.reason());
```

## Layers

Every vocabulary is one of four things, and confusing them is how a data model
ends up owned by somebody else's release cycle:

| Layer | Relationship | Example |
|---|---|---|
| `foundational` | Something you **build on** | GS1 Web Vocabulary, EU SEMIC Core Vocabularies |
| `community-profile` | Something you **map to** | UN Transparency Protocol, CIRPASS-2 |
| `sectoral` | A domain vocabulary beside the foundation | GS1 Rail |
| `conformance-bridge` | Something you are **tested by** | CEN/CENELEC JTC 24, Battery Pass SAMM |

A conformance bridge is never speakable. Being measured against a model is not
the same as speaking its language, and adopting a bridge's identifiers converts
an external check into an internal dependency.

## Adding a vocabulary

One JSON file in `vocabularies/`, one entry in the embedded list. Never two
authorities in one file — a reader would have no way to tell which one a
`finding` belonged to. The fixed key set and the meaning of each status are
documented in `vocabularies/README.md`.

## The mapping layer, and the standard it will use

This crate records **vocabularies**. The layer above it records **mappings** —
"our term X corresponds to their term Y" — and that layer does not exist yet.

When it is built it adopts [SSSOM](https://arxiv.org/abs/2112.07051) (A Simple
Standard for Sharing Ontological Mappings) rather than inventing a shape. SSSOM
requires four slots per mapping — `subject_id`, `object_id`, `predicate_id` and
**`match_type`** — and recommends `predicate_id` be drawn from SKOS or OWL:
`skos:exactMatch`, `skos:closeMatch`, `skos:broadMatch`, `skos:narrowMatch`.

`match_type` is the slot that matters most here, because it records *how a
mapping was derived*, and SSSOM further says a match should reference the
`mapping_tool` and `mapping_tool_version` that produced it. That is precisely
the distinction this project needs and had been planning to express in its own
words: a mapping asserted by a machine pipeline and a mapping asserted by a
person who read the specification are different claims, and a standard already
exists for saying so.

The distinction between `exactMatch` and `closeMatch` also does real work.
`exactMatch` is transitive — chain three of them and the result still holds —
while `closeMatch` deliberately is not, precisely to stop compound errors
accumulating across combined mappings. Borrowing a term because it is
"close enough" is exactly the case `closeMatch` exists to mark, and exactly the
case that must never be recorded as `exactMatch`.

SSSOM also requires identifiers to be CURIEs with a registered prefix, which is
what the `namespaceIri` field in each record exists to supply.

## What this crate does not do

It records **identity provenance**, not structural conformance. That an
identifier is well-formed, current and correctly attributed says nothing about
whether our field set matches the model behind it. Nothing here licenses the
phrase "IDTA-conformant", "JTC 24 conformant", or any equivalent — the EN 18216
series carries no OJEU citation, so there is no presumption of conformity to
claim even once a text has been read.

## Design

A **leaf crate**: no workspace dependencies, by design. Everything else in this
workspace follows one rule — *if it changes because an EU regulation changed, it
belongs here.* This crate does not. Its contents change when GS1 or IDTA
publishes, on a different authority's clock, which is why it sits apart from the
passport model rather than inside it.

If this crate ever needs `dpp-domain`, that dependency is the signal that
something passport-shaped has leaked into it.

## Licence

Apache-2.0. Note that the vocabularies *described* here carry their own terms:
several are unstated, at least one is CC BY 4.0 with an attribution obligation,
and the JTC 24 standards are purchased and may not be redistributed. The records
store identifiers and citations, never an authority's prose.
