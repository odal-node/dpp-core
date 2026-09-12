//! `dpp-digital-link` — GS1 Digital Link parsing and building, and GS1 link-type
//! negotiation.
//!
//! Pure, stateless crate with no I/O or network dependencies. Compiles to both
//! `std` and `wasm32`.
//!
//! # 🔶 Which version of the URI syntax this implements is not established
//!
//! **EN 18219:2026** — *Digital product passport: Unique identifiers* — is one
//! of the six harmonised standards cited by Commission Implementing Decision
//! (EU) 2026/1736, and **Art. 41(2) of Regulation (EU) 2024/1781** makes
//! conformity with it a presumption of conformity with that Regulation's
//! Articles 10 and 11.
//!
//! Its clause 5.1 requires a unique product identifier to comply with one of the
//! ID schemes in its Clause 5. The scheme the `/01/{gtin}/21/{serial}` form
//! produced here sits in — scheme 1, web-enabled structured-path identification
//! — requires conformance with the **GS1 Digital Link URI Syntax standard at a
//! specifically named version**, and clause 6.3.2 names **1.6.0:2022**.
//!
//! This crate has never named a version, and that is the defect: **a versioned
//! normative requirement cannot be met by an unversioned implementation claim.**
//! Not because the parser is wrong — as far as review has gone it is fine — but
//! because nobody can check it, and it cannot go stale visibly. When GS1
//! publishes the next revision, nothing here will indicate whether this still
//! conforms.
//!
//! ⚠️ **Naming 1.6.0:2022 here would be asserting something unverified.** The
//! parser and builder have not been diffed against that revision, and a separate
//! conformity document in this repository has long claimed *v1.2* — an older
//! revision — without recording where that claim came from. So there are two
//! candidate versions, no evidence for either, and the honest state is that the
//! question is open. It is recorded here rather than left invisible.
//!
//! **Resolving it means diffing the parser and builder against 1.6.0:2022 and
//! naming the outcome in this comment.** If the crate turns out to implement an
//! older revision than EN 18219 names, that is a larger finding than a missing
//! version string — and it is not one this note can pre-empt.
//!
//! This crate previously also carried AAS mapping and a JSON-LD context behind
//! its name. Both have moved out — the AAS projection to [`dpp-aas`], the
//! JSON-LD context to `dpp-vc` — and neither is reachable from here, which is
//! the point: a consumer that wants a GS1 parser no longer compiles an AAS
//! mapper to get one.
//!
//! [`dpp-aas`]: https://docs.rs/dpp-aas

pub mod digital_link;
pub mod linktype;

pub use digital_link::{
    AiSpec, DigitalLink, DigitalLinkError, ElementString, PrimaryKey, ai_len_for_prefix, ai_spec,
    build_qr_url, dictionary, qualifier_position, short_serial, validate_gtin,
};
pub use linktype::{
    Audience, DppMediaType, Gs1LinkType, LinkDescriptor, ResolutionRequest, negotiate,
};

/// Compile-checks this crate's README examples.
///
/// A README example is a public claim about the API, and nothing else in the
/// build compiles one. Without this, a README can advertise a function that
/// does not exist — which is exactly what happened before this harness landed.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
