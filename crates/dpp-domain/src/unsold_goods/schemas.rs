//! The versioned JSON Schemas of the unsold-goods disclosure.
//!
//! # A schema family of its own
//!
//! These schemas are not product-group schemas and are not in
//! [`VersionedSchemaRegistry::new`], whose every entry is read as the data of a
//! product group: held there, a disclosure would be listed beside the passports'
//! schemas, linted as one, and given a JSON-LD vocabulary scope as one. They are
//! held in a registry of their own, built by [`document_schemas`], and validated
//! by the same machinery:
//!
//! ```rust,ignore
//! let schemas = dpp_domain::unsold_goods::document_schemas();
//! let version = "2.0.0".parse()?;
//! schemas.validate(DISCLOSURE_SCHEMA_KEY, &version, &body)?;
//! ```
//!
//! # The version numbers are kept
//!
//! v2.0.0 moved out of the product-group schemas byte for byte. The shape is the
//! same, the schema's `$id` is the one already published, and a published schema
//! version is never modified, so it is neither renumbered nor republished as a
//! first version. The key is kept for the same reason.
//!
//! v2.0.0 is the format Impl. Reg. (EU) 2026/2 Annex I prescribes. v2.1.0 stops
//! requiring a reason on a line, since Annex I note (h) asks for one only where a
//! derogation applies; dropping a requirement only widens what validates, so a
//! v2.0.0 disclosure reaches it unchanged.
//!
//! There is no v1.0.0. It predated Impl. Reg. (EU) 2026/2 and nothing can carry a
//! document forward from it: a financial year is not derivable from a quarter, a
//! CN code is not derivable from the word "apparel", a six-way treatment split is
//! not derivable from one destination, and its reason list has no member in common
//! with the Art. 2 derogations. A lens would have to invent every one of those, so
//! the version was removed rather than migrated. Safe only because nothing has ever
//! been stored under it.

use semver::Version;

use crate::schemas::{SchemaEntry, SchemaOrigin, VersionedSchemaRegistry};

/// The key the disclosure's schema versions are held under.
pub const DISCLOSURE_SCHEMA_KEY: &str = "unsold-goods";

/// Every version of the disclosure schema, compiled into the crate so that a
/// published crate carries its own schemas.
const EMBEDDED: &[(&str, &str)] = &[
    (
        "2.0.0",
        include_str!("../../document-schemas/unsold-goods/v2.0.0.json"),
    ),
    (
        "2.1.0",
        include_str!("../../document-schemas/unsold-goods/v2.1.0.json"),
    ),
];

/// A registry holding the disclosure's schema versions, and nothing else.
#[must_use]
pub fn document_schemas() -> VersionedSchemaRegistry {
    VersionedSchemaRegistry::from_entries(
        EMBEDDED
            .iter()
            .map(|(version, json)| SchemaEntry {
                product_group: DISCLOSURE_SCHEMA_KEY.to_owned(),
                version: version
                    .parse::<Version>()
                    .expect("embedded schema version is valid semver"),
                json: (*json).to_owned(),
                origin: SchemaOrigin::Embedded,
            })
            .collect(),
    )
}
