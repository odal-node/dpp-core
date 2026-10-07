//! GS1 Digital Link parser, builder, and GTIN utilities.
//!
//! A passport's carrier: `{resolver}/01/{gtin}`, followed by a batch or a
//! carrier serial according to the level the passport describes — see
//! [`build_qr_url`], and [`DigitalLink::carrier_qualifier`] for reading one back.
//!
//! Reads and builds the path of a GS1 Digital Link URI, against GS1 Digital Link
//! URI Syntax 1.7.0, section 4. The crate-level documentation says what that
//! covers and where it departs.
//!
//! Application Identifiers (AIs) recognised in the path:
//! - any of GS1's sixteen `dlpkey` AIs as the primary key — `01` (GTIN) is
//!   parsed into a validated [`dpp_domain::Gtin`], the rest are carried as
//!   [`PrimaryKey::Other`]. See that type for why only one is validated.
//! - `01`  — GTIN-14 (the legacy GTIN-8/12/13 are padded to 14 on reading, and
//!   never written)
//! - `22`  — Consumer product variant (qualifier; canonical order 1)
//! - `10`  — Batch/lot number (qualifier; canonical order 2)
//! - `21`  — Serial number (qualifier; canonical order 3)
//! - `235` — Third-party controlled serial (qualifier; canonical order 4)
//!
//! Query parameters (`?…`) are split from the path before segmenting so they
//! can never corrupt the value of the last qualifier.
//! AI values are percent-decoded on parse and percent-encoded on build.
//! The resolver base URL preserves any path prefix that precedes the primary-key
//! segment, so `https://example.com/resolve/01/…` round-trips correctly.
//!
//! ## Module layout
//!
//! - `syntax_dictionary` — GS1's published Syntax Dictionary, parsed. The single
//!   authority for every AI's length, its pre-defined-length flag, and which
//!   qualifier sequences a primary key accepts. Both the URI parser and the
//!   element-string reader derive from it; there is no second, hand-written
//!   table beside it.
//! - `component` — [`Component`] and [`CharKind`], one component of an AI's
//!   specification and the character set it draws from.
//! - `element_string` — [`ElementString`], the AI data a scanner emits.
//! - `error` — [`DigitalLinkError`].
//! - `codec`   — percent-encode/decode, the host and port a Web URI can have, and
//!   GTIN normalisation (private helpers).
//! - `value`   — holds one AI value to its dictionary entry: length, the
//!   character set of each component, and the check digit where one is named.
//! - `link`   — [`DigitalLink`] (parse/build).
//! - `primary_key` — [`PrimaryKey`], the AI a path opens on.
//! - `gtin`   — [`validate_gtin`].
//! - `qr`     — [`build_qr_url`], a passport's data carrier.

mod codec;
mod component;
mod element_string;
mod error;
mod gtin;
mod link;
mod primary_key;
mod qr;
mod syntax_dictionary;
#[cfg(test)]
mod tests;
mod value;
#[cfg(test)]
mod value_tests;

pub use component::{CharKind, Component};
pub use element_string::ElementString;
pub use error::DigitalLinkError;
pub use gtin::validate_gtin;
pub use link::DigitalLink;
pub use primary_key::PrimaryKey;
pub use qr::build_qr_url;
pub use syntax_dictionary::{
    AiSpec, ai_len_for_prefix, ai_spec, dictionary, qualifier_position, required_qualifier,
};
