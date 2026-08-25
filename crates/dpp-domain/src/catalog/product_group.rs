//! [`ProductGroupCatalog`] — the open, data-driven product group catalog, pre-loaded from
//! embedded manifests and extensible at runtime.

use super::descriptor::ProductGroupDescriptor;
use super::error::CatalogError;

struct EmbeddedManifest {
    key: &'static str,
    json: &'static str,
}

/// One manifest per product group. Adding a product group at compile time is a single entry +
/// a JSON file; adding one at runtime is [`ProductGroupCatalog::register`].
const EMBEDDED: &[EmbeddedManifest] = &[
    EmbeddedManifest {
        key: "battery",
        json: include_str!("../../product-groups/battery.json"),
    },
    EmbeddedManifest {
        key: "electronics",
        json: include_str!("../../product-groups/electronics.json"),
    },
    EmbeddedManifest {
        key: "unsold-goods",
        json: include_str!("../../product-groups/unsold-goods.json"),
    },
    EmbeddedManifest {
        key: "textile",
        json: include_str!("../../product-groups/textile.json"),
    },
    EmbeddedManifest {
        key: "steel",
        json: include_str!("../../product-groups/steel.json"),
    },
    EmbeddedManifest {
        key: "construction",
        json: include_str!("../../product-groups/construction.json"),
    },
    EmbeddedManifest {
        key: "tyre",
        json: include_str!("../../product-groups/tyre.json"),
    },
    EmbeddedManifest {
        key: "toy",
        json: include_str!("../../product-groups/toy.json"),
    },
    EmbeddedManifest {
        key: "aluminium",
        json: include_str!("../../product-groups/aluminium.json"),
    },
    EmbeddedManifest {
        key: "furniture",
        json: include_str!("../../product-groups/furniture.json"),
    },
    EmbeddedManifest {
        key: "detergent",
        json: include_str!("../../product-groups/detergent.json"),
    },
    EmbeddedManifest {
        key: "mattress",
        json: include_str!("../../product-groups/mattress.json"),
    },
];

/// Open, data-driven product-group catalog. Pre-loaded with embedded manifests
/// and extensible at runtime.
///
/// Identity and scope only. Anything a caller wants to know about the **law** —
/// whether obligations bind, whether a passport is owed and from when, how long
/// a record must be kept, at what level — comes from
/// [`InstrumentCatalog`](crate::domain::instrument::InstrumentCatalog), because each of those is a
/// property of an act reaching this group and a group may be reached by several.
pub struct ProductGroupCatalog {
    entries: Vec<ProductGroupDescriptor>,
}

impl ProductGroupCatalog {
    /// Create a catalog pre-loaded with all embedded product group manifests.
    #[must_use]
    pub fn new() -> Self {
        let entries = EMBEDDED
            .iter()
            .map(|m| {
                let descriptor: ProductGroupDescriptor = serde_json::from_str(m.json)
                    .unwrap_or_else(|e| {
                        panic!(
                            "embedded product_group manifest '{}' is invalid: {e}",
                            m.key
                        )
                    });
                assert_eq!(
                    descriptor.key, m.key,
                    "manifest key '{}' does not match its file key '{}'",
                    descriptor.key, m.key
                );
                descriptor
            })
            .collect();
        Self { entries }
    }

    /// Look up a product group by canonical key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&ProductGroupDescriptor> {
        self.entries.iter().find(|d| d.key == key)
    }

    /// All product group descriptors.
    #[must_use]
    pub fn all(&self) -> &[ProductGroupDescriptor] {
        &self.entries
    }

    /// The schema version applicable to *new* passports in `key`.
    #[must_use]
    pub fn current_schema_version(&self, key: &str) -> Option<&str> {
        self.get(key).map(|d| d.current_schema_version.as_str())
    }

    /// Resolve which schema version to validate against — the one mechanism that
    /// replaces hardcoded `"1.0.0"` / `latest()` at call sites.
    ///
    /// - `stored = Some(v)` (an *existing* passport): that version is
    ///   authoritative — a record is always re-validated against the version it
    ///   was published under, for immutability and audit. Returned as-is.
    /// - `stored = None` (a *new* passport): the product group's current version from
    ///   the catalog is used.
    ///
    /// Returns `None` only if `stored` is `None` and the product group is unknown.
    #[must_use]
    pub fn resolve_schema_version(&self, key: &str, stored: Option<&str>) -> Option<String> {
        match stored {
            Some(v) => Some(v.to_owned()),
            None => self.current_schema_version(key).map(ToOwned::to_owned),
        }
    }

    /// All product group keys, sorted.
    #[must_use]
    pub fn keys(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = self.entries.iter().map(|d| d.key.as_str()).collect();
        keys.sort_unstable();
        keys
    }

    /// Register a new product group at runtime.
    ///
    /// Enforces the descriptor invariant the schema-resolution path relies on:
    /// `current_schema_version` must be valid semver **and** one of
    /// `schema_versions`. A descriptor violating either would otherwise let a
    /// caller register a product group whose version fails to parse downstream, which
    /// silently skips JSON-Schema validation for every passport in that product group.
    ///
    /// # Errors
    /// - [`CatalogError::AlreadyExists`] if the key is already taken.
    /// - [`CatalogError::InvalidSchemaVersion`] if `current_schema_version` is
    ///   not valid semver.
    /// - [`CatalogError::CurrentVersionNotListed`] if `current_schema_version`
    ///   is not present in `schema_versions`.
    pub fn register(&mut self, descriptor: ProductGroupDescriptor) -> Result<(), CatalogError> {
        if self.get(&descriptor.key).is_some() {
            return Err(CatalogError::AlreadyExists(descriptor.key));
        }
        if descriptor
            .current_schema_version
            .parse::<semver::Version>()
            .is_err()
        {
            return Err(CatalogError::InvalidSchemaVersion {
                key: descriptor.key,
                version: descriptor.current_schema_version,
            });
        }
        if !descriptor
            .schema_versions
            .contains(&descriptor.current_schema_version)
        {
            return Err(CatalogError::CurrentVersionNotListed {
                key: descriptor.key,
                version: descriptor.current_schema_version,
            });
        }
        self.entries.push(descriptor);
        Ok(())
    }

    /// Number of product groups in the catalog.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the catalog is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for ProductGroupCatalog {
    fn default() -> Self {
        Self::new()
    }
}
