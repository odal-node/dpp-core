//! Upstream vocabulary register for Digital Product Passports.
//!
//! Naming another organisation's term asserts, **to a machine**, that our field
//! means what that organisation says its identifier means. This crate is the one
//! home for that class of claim: who publishes a vocabulary, under what licence,
//! and whether a person here has actually read it.
//!
//! # The rule
//!
//! > An identifier is either in the `urn:odal-node:` namespace, or it belongs to
//! > a vocabulary somebody has read. There is deliberately no third category.
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
//! **No vocabulary is verified.** Every third-party identifier is therefore
//! refused. That is the accurate state of this project's knowledge rather than
//! a gap waiting to be filled by default — the twelve records in
//! `vocabularies/` each carry the finding that got them to their status and the
//! step that would move them on.
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

pub use namespace::{OWN_NAMESPACE, is_own};
pub use register::{Verdict, VocabularyRegister};
pub use vocabulary::{Layer, Status, Vocabulary};
