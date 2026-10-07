//! `dpp-digital-link` — GS1 Digital Link parsing and building, and GS1 link-type
//! negotiation.
//!
//! Pure, stateless crate with no I/O or network dependencies. Compiles to both
//! `std` and `wasm32`.
//!
//! # Which version of the URI syntax this implements
//!
//! **GS1 Digital Link URI Syntax 1.7.0**, section 4, the ABNF grammar that
//! section 2 of that standard says defines conformance. The parser and builder
//! were read against it rule by rule, and a corpus with one URI for each rule is
//! judged by the GS1 Barcode Syntax Engine. That engine is a component of the GS1
//! Barcode Syntax Resource, which section 2 names as a tool for confirming
//! conformance.
//!
//! **EN 18219:2026** — *Digital product passport: Unique identifiers* — is one
//! of the six harmonised standards cited by Commission Implementing Decision
//! (EU) 2026/1736, and Art. 41(2) of Regulation (EU) 2024/1781 makes conformity
//! with it a presumption of conformity with that Regulation's Articles 10 and 11.
//! Its identifier scheme 1, which the `/01/{gtin}/21/{serial}` form produced here
//! sits in, requires the Digital Link URI Syntax at a named revision, and clause
//! 6.3.2 names **1.6.0:2022**. A claim against 1.7.0 does not by itself show
//! conformance with that clause. GS1's change log for 1.7.0 lists five changes
//! since 1.6.0, and only the data attributes added for new Application
//! Identifiers touch the grammar, in the query string, which this crate does not
//! read. Whether that settles the question is a matter for the harmonised
//! standard's assessment, not for this crate.
//!
//! # Where the reader departs from the grammar
//!
//! Each of these is recorded against the case that shows it in
//! `tests/gs1_syntax_rules_corpus.rs`, and none affects what the builder writes.
//!
//! - **A GTIN of fewer than 14 digits is read and padded.** The grammar wants 14.
//!   GS1 says only existing infrastructure should keep reading the legacy forms,
//!   which is what labels already printed carry.
//! - **A trailing slash is tolerated.** It is not in the grammar, and GS1's
//!   resolver standard asks resolvers to accept it.
//! - **A symbol the grammar spells as an escape may be written raw.** `!`, `&`,
//!   `'`, `(`, `)`, `*`, `+`, `,`, `;`, `=` and `:` are legal in a path under
//!   RFC 3986. They are always built escaped.
//! - **The double quote is built as `%22`.** The grammar names a raw `"`, which
//!   RFC 3986 does not allow in a URI and GS1's engine refuses.
//! - **The query string is not read.** It is cut off at the first `?` so it can
//!   never corrupt the last path value, and what it holds is not validated:
//!   neither a data attribute's format nor the shape of an extension parameter.
//! - **An empty host is refused.** The grammar's `reg-name` may be empty, which
//!   names no resolver and cannot be built back into a link.
//! - **GS1's deeper validation is not run.** Lengths, character sets and the
//!   modulo-10 check digit are applied from the dictionary. That the digits begin
//!   with a plausible GS1 Company Prefix, and the check character pair of a Global
//!   Model Number, are not, and the other linters the dictionary names are not
//!   either.
//! - **A custom path is read by finding the primary key.** The first segment that
//!   names one opens the path, so a resolver's own path prefix that contains a
//!   primary-key AI as a whole segment is misread. GS1 says a custom path cannot
//!   be tested by the grammar at all.
//! - **Compressed Digital Link URIs are not read.**
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
    AiSpec, CharKind, Component, DigitalLink, DigitalLinkError, ElementString, PrimaryKey,
    ai_len_for_prefix, ai_spec, build_qr_url, dictionary, qualifier_position, required_qualifier,
    validate_gtin,
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
