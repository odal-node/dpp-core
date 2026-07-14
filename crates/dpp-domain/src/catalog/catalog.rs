//! [`SectorCatalog`] — the open, data-driven sector catalog, pre-loaded from
//! embedded manifests and extensible at runtime.

use super::descriptor::SectorDescriptor;
use super::error::CatalogError;
use super::status::RegulatoryStatus;

struct EmbeddedManifest {
    key: &'static str,
    json: &'static str,
}

/// One manifest per sector. Adding a sector at compile time is a single entry +
/// a JSON file; adding one at runtime is [`SectorCatalog::register`].
const EMBEDDED: &[EmbeddedManifest] = &[
    EmbeddedManifest {
        key: "battery",
        json: include_str!("../../sectors/battery.json"),
    },
    EmbeddedManifest {
        key: "electronics",
        json: include_str!("../../sectors/electronics.json"),
    },
    EmbeddedManifest {
        key: "unsold-goods",
        json: include_str!("../../sectors/unsold-goods.json"),
    },
    EmbeddedManifest {
        key: "textile",
        json: include_str!("../../sectors/textile.json"),
    },
    EmbeddedManifest {
        key: "steel",
        json: include_str!("../../sectors/steel.json"),
    },
    EmbeddedManifest {
        key: "construction",
        json: include_str!("../../sectors/construction.json"),
    },
    EmbeddedManifest {
        key: "tyre",
        json: include_str!("../../sectors/tyre.json"),
    },
    EmbeddedManifest {
        key: "toy",
        json: include_str!("../../sectors/toy.json"),
    },
    EmbeddedManifest {
        key: "aluminium",
        json: include_str!("../../sectors/aluminium.json"),
    },
    EmbeddedManifest {
        key: "furniture",
        json: include_str!("../../sectors/furniture.json"),
    },
    EmbeddedManifest {
        key: "detergent",
        json: include_str!("../../sectors/detergent.json"),
    },
];

/// Open, data-driven sector catalog. Pre-loaded with embedded manifests and
/// extensible at runtime.
pub struct SectorCatalog {
    entries: Vec<SectorDescriptor>,
}

impl SectorCatalog {
    /// Create a catalog pre-loaded with all embedded sector manifests.
    #[must_use]
    pub fn new() -> Self {
        let entries = EMBEDDED
            .iter()
            .map(|m| {
                let descriptor: SectorDescriptor =
                    serde_json::from_str(m.json).unwrap_or_else(|e| {
                        panic!("embedded sector manifest '{}' is invalid: {e}", m.key)
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

    /// Look up a sector by canonical key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&SectorDescriptor> {
        self.entries.iter().find(|d| d.key == key)
    }

    /// All sector descriptors.
    #[must_use]
    pub fn all(&self) -> &[SectorDescriptor] {
        &self.entries
    }

    /// Sectors whose DPP obligation is in force (determinations are binding).
    #[must_use]
    pub fn in_force(&self) -> Vec<&SectorDescriptor> {
        self.entries
            .iter()
            .filter(|d| d.status == RegulatoryStatus::InForce)
            .collect()
    }

    /// Sectors that are flagged provisional (no binding determinations).
    #[must_use]
    pub fn provisional(&self) -> Vec<&SectorDescriptor> {
        self.entries
            .iter()
            .filter(|d| d.status == RegulatoryStatus::Provisional)
            .collect()
    }

    /// Whether the sector exists and may carry a binding determination.
    #[must_use]
    pub fn is_in_force(&self, key: &str) -> bool {
        self.get(key)
            .is_some_and(|d| d.status.allows_determination())
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
    /// - `stored = None` (a *new* passport): the sector's current version from
    ///   the catalog is used.
    ///
    /// Returns `None` only if `stored` is `None` and the sector is unknown.
    #[must_use]
    pub fn resolve_schema_version(&self, key: &str, stored: Option<&str>) -> Option<String> {
        match stored {
            Some(v) => Some(v.to_owned()),
            None => self.current_schema_version(key).map(ToOwned::to_owned),
        }
    }

    /// All sector keys, sorted.
    #[must_use]
    pub fn keys(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = self.entries.iter().map(|d| d.key.as_str()).collect();
        keys.sort_unstable();
        keys
    }

    /// Register a new sector at runtime.
    ///
    /// Enforces the descriptor invariant the schema-resolution path relies on:
    /// `current_schema_version` must be valid semver **and** one of
    /// `schema_versions`. A descriptor violating either would otherwise let a
    /// caller register a sector whose version fails to parse downstream, which
    /// silently skips JSON-Schema validation for every passport in that sector.
    ///
    /// # Errors
    /// - [`CatalogError::AlreadyExists`] if the key is already taken.
    /// - [`CatalogError::InvalidSchemaVersion`] if `current_schema_version` is
    ///   not valid semver.
    /// - [`CatalogError::CurrentVersionNotListed`] if `current_schema_version`
    ///   is not present in `schema_versions`.
    pub fn register(&mut self, descriptor: SectorDescriptor) -> Result<(), CatalogError> {
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

    /// Number of sectors in the catalog.
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

impl Default for SectorCatalog {
    fn default() -> Self {
        Self::new()
    }
}
