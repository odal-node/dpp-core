//! GS1 Digital Link parser, builder, and GTIN utilities.
//!
//! Canonical Odal form: `https://id.odal-node.io/01/{gtin}/21/{serial}`
//!
//! Supports the GS1 Digital Link standard (GS1 DL URI Syntax, v1.2).
//! Application Identifiers (AIs) recognised in the path:
//! - any of GS1's sixteen `dlpkey` AIs as the primary key — `01` (GTIN) is
//!   parsed into a validated [`dpp_domain::Gtin`], the rest are carried as
//!   [`PrimaryKey::Other`]. See that type for why only one is validated.
//! - `01`  — GTIN-14 (GTIN-8/12/13 normalised to 14 by left-padding)
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
//! - `element_string` — [`ElementString`], the AI data a scanner emits.
//! - `error` — [`DigitalLinkError`].
//! - `codec`   — percent-encode/decode and GTIN normalisation (private helpers).
//! - `link`   — [`DigitalLink`] (parse/build).
//! - `primary_key` — [`PrimaryKey`], the AI a path opens on.
//! - `gtin`   — [`validate_gtin`].
//! - `qr`     — [`build_qr_url`].

mod codec;
mod element_string;
mod error;
mod gtin;
mod link;
mod primary_key;
mod qr;
mod syntax_dictionary;
#[cfg(test)]
mod tests;

pub use element_string::ElementString;
pub use error::DigitalLinkError;
pub use gtin::validate_gtin;
pub use link::DigitalLink;
pub use primary_key::PrimaryKey;
pub use qr::{build_qr_url, short_serial};
pub use syntax_dictionary::{AiSpec, ai_len_for_prefix, ai_spec, dictionary, qualifier_position};
