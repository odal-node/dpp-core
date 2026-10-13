//! Upstream vocabulary register for Digital Product Passports.
//!
//! Naming another organisation's term asserts, **to a machine**, that our field
//! means what that organisation says its identifier means. This crate is the one
//! home for that class of claim: who publishes a vocabulary, under what licence,
//! and whether a person here has actually read it.
//!
//! # The rule
//!
//! > An identifier is either in the `urn:odal-node:` namespace, or it is a term
//! > somebody has read, in a vocabulary somebody has read. There is deliberately
//! > no third category.
//!
//! The two readings are separate. A namespace is a prefix, so every string that
//! begins with it looks like one of the vocabulary's terms, and reading the
//! vocabulary's publication does not read each definition in it. Each record
//! therefore lists the terms read, and only a listed term is permitted.
//!
//! Declining to make a claim beats making one we cannot support. An honest
//! `urn:odal-node:` identifier says *"this is our concept"*; a wrong IRDI says
//! *"this is ECLASS's concept"*, falsely, in the format most likely to be
//! consumed without a human ever looking at it.
//!
//! # Why this is a crate and not a module
//!
//! Everything else in `dpp-core` follows one rule: *if it changes because an EU
//! regulation changed, it belongs here*. This crate does not. Its contents
//! change when **GS1 or IDTA publishes** — a different authority on a different
//! clock — which is why it sits apart from the passport model rather than
//! inside it. It is a leaf: no workspace dependencies, by design.
//!
//! # What it does not do
//!
//! It records **identity provenance**, not structural conformance. That an
//! identifier is well-formed, current and correctly attributed says nothing
//! about whether our field set matches the model behind it. Nothing here
//! licenses the phrase "IDTA-conformant" or any equivalent.
//!
//! # Current state
//!
//! Six of the twenty records in `vocabularies/` are verified (`batterypass-samm`,
//! `catena-x-battery-pass`, `ec-battery-guidance`, `gs1`, `idta`, `semic`): each
//! because a person read the authority's own publication and recorded what it said,
//! on 2026-08-11, and each checked again for change on 2026-10-13. Five of the six
//! now carry a re-check note recording a newer version, a successor or a correction,
//! which is why they are re-checked. Everything else stays refused. Each record carries the finding that
//! got it to its status and the step that would move it on.
//!
//! One term has been read so far, GS1's `gtin`. A verified vocabulary whose `terms`
//! is empty permits no term, so reading another GS1 term is a separate task.
//!
//! ```
//! use dpp_vocab::{VocabularyRegister, Verdict};
//!
//! let register = VocabularyRegister::new();
//!
//! assert_eq!(register.verdict("urn:odal-node:aas:property:co2e-per-unit:1.0"), Verdict::Own);
//!
//! // A conformance bridge is something we are tested by, not something we speak.
//! let v = register.verdict("urn:samm:io.catenax.battery.battery_pass:6.0.0#BatteryPass");
//! assert!(!v.is_permitted());
//! ```

#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

pub mod namespace;
pub mod register;
pub mod vocabulary;

pub use namespace::{OWN_JSONLD_NAMESPACE, OWN_NAMESPACE, is_own};
pub use register::{Verdict, VocabularyRegister};
pub use vocabulary::{Layer, ReadTerm, Status, Vocabulary};
