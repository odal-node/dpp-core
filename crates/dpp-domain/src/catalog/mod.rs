//! Open, data-driven catalog of EU ESPR sectors.
//!
//! The catalog is the single source of truth for *what sectors exist* and
//! *where each stands in the EU regulatory pipeline*. Unlike a closed `enum`,
//! sectors are described by **data** — one embedded manifest per sector at
//! `dpp-core/crates/dpp-domain/sectors/{key}.json` — and new sectors can be
//! added at runtime via [`SectorCatalog::register`] without recompiling core.
//!
//! Each [`SectorDescriptor`] ties together a sector's canonical key, regulatory
//! status, legal basis, schema versions, retention, product categories, and
//! plugin binding — resolving the "four spellings of a sector" problem by
//! giving every component one record to agree on.
//!
//! [`RegulatoryStatus`] gates behaviour: only `InForce` sectors may carry a
//! binding compliance determination. `Provisional` sectors (on the ESPR working
//! plan but without an adopted delegated act) are present but **flagged** —
//! their schemas are best-effort drafts and plugins must not assert
//! COMPLIANT/NON_COMPLIANT.

use serde::{Deserialize, Serialize};

// ─── Embedded manifests ───────────────────────────────────────────────────────

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
        key: "textile-unsold",
        json: include_str!("../../sectors/textile-unsold.json"),
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

// ─── Regulatory status ────────────────────────────────────────────────────────

/// Where a sector's DPP obligation stands in the EU regulatory pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RegulatoryStatus {
    /// A DPP / ecodesign obligation is legally in force, or has a firm adopted
    /// applicability date. Plugins may emit binding compliance determinations.
    InForce,
    /// On the ESPR working plan, or a delegated act is anticipated, but no DPP
    /// obligation is in force yet. Schemas are best-effort drafts; plugins must
    /// not assert COMPLIANT/NON_COMPLIANT — only structural validation applies.
    Provisional,
}

impl RegulatoryStatus {
    /// Whether a sector with this status may carry a *binding* compliance
    /// determination (vs. structural validation only).
    #[must_use]
    pub fn allows_determination(&self) -> bool {
        matches!(self, Self::InForce)
    }
}

// ─── Sector descriptor ────────────────────────────────────────────────────────

/// A single sector's catalog entry — the canonical record every component
/// (schema registry, plugin host, passport model) resolves against.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectorDescriptor {
    /// Canonical sector key, e.g. `"battery"`, `"textile-unsold"`. Matches the
    /// schema-registry sector key and the plugin's `meta().sector`.
    pub key: String,
    /// Human-readable title.
    pub title: String,
    /// Regulatory status — gates whether determinations are binding.
    pub status: RegulatoryStatus,
    /// EU legal instrument(s) this sector derives from.
    pub legal_basis: Vec<String>,
    /// ISO-8601 date the DPP obligation applies from, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dpp_applies_from: Option<String>,
    /// Minimum data retention in years required by the applicable act.
    pub retention_years: u32,
    /// Schema versions available for this sector (semver strings).
    pub schema_versions: Vec<String>,
    /// The schema version applicable to *new* passports in this sector right
    /// now. Decouples "current" from "latest embedded" so a future schema can
    /// ship embedded without becoming current until its act is in force. Must
    /// be one of `schema_versions`.
    pub current_schema_version: String,
    /// Product categories *within* this sector — sub-types a plugin may branch
    /// on, never dispatch keys. See `DATA-MODEL.md` §3.5.
    #[serde(default)]
    pub product_categories: Vec<String>,
    /// Per-field minimum ESPR access tier (public/professional/confidential) for
    /// this sector's data: field name → tier; unlisted fields default to public.
    /// Universal confidential fields (signatures, audit trails) are folded in by
    /// the access-policy engine, so they are not repeated per sector here.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub access_tiers: std::collections::HashMap<String, crate::domain::identity::AccessTier>,
    /// Plugin that handles this sector (crate / filename stem, e.g.
    /// `"sector-battery"`). `None` if no plugin is bound yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    /// Free-text regulatory note (effective dates, scope, caveats).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

// ─── Errors ───────────────────────────────────────────────────────────────────

/// Errors from runtime catalog registration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CatalogError {
    /// A descriptor for this key already exists.
    AlreadyExists(String),
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyExists(key) => write!(f, "sector '{key}' already in catalog"),
        }
    }
}

impl std::error::Error for CatalogError {}

// ─── Catalog ──────────────────────────────────────────────────────────────────

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

    /// Register a new sector at runtime. Returns `AlreadyExists` if the key is
    /// taken.
    pub fn register(&mut self, descriptor: SectorDescriptor) -> Result<(), CatalogError> {
        if self.get(&descriptor.key).is_some() {
            return Err(CatalogError::AlreadyExists(descriptor.key));
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

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_all_embedded_manifests() {
        let catalog = SectorCatalog::new();
        assert_eq!(catalog.len(), 11);
    }

    #[test]
    fn exactly_three_sectors_are_in_force() {
        let catalog = SectorCatalog::new();
        let mut in_force: Vec<&str> = catalog.in_force().iter().map(|d| d.key.as_str()).collect();
        in_force.sort_unstable();
        assert_eq!(in_force, vec!["battery", "electronics", "textile-unsold"]);
    }

    #[test]
    fn provisional_sectors_are_flagged_not_dropped() {
        let catalog = SectorCatalog::new();
        // All eight not-yet-adopted sectors are still present, just flagged.
        assert_eq!(catalog.provisional().len(), 8);
        assert!(!catalog.is_in_force("textile"));
        assert!(!catalog.is_in_force("steel"));
    }

    #[test]
    fn battery_descriptor_is_complete() {
        let catalog = SectorCatalog::new();
        let battery = catalog.get("battery").expect("battery in catalog");
        assert_eq!(battery.status, RegulatoryStatus::InForce);
        assert_eq!(battery.dpp_applies_from.as_deref(), Some("2027-02-18"));
        assert_eq!(battery.retention_years, 10);
        assert!(battery.schema_versions.contains(&"2.0.0".to_string()));
        // Current version is v2.0.0 (Annex XIII), not the older v1.0.0.
        assert_eq!(battery.current_schema_version, "2.0.0");
        assert_eq!(battery.plugin.as_deref(), Some("sector-battery"));
    }

    #[test]
    fn resolve_schema_version_new_vs_existing() {
        let catalog = SectorCatalog::new();
        // New passport (stored = None) → catalog current version.
        assert_eq!(
            catalog.resolve_schema_version("battery", None).as_deref(),
            Some("2.0.0")
        );
        // Existing passport → its stored version is authoritative, even if old.
        assert_eq!(
            catalog
                .resolve_schema_version("battery", Some("1.0.0"))
                .as_deref(),
            Some("1.0.0")
        );
        // Unknown sector, new passport → None.
        assert_eq!(catalog.resolve_schema_version("unknown", None), None);
    }

    #[test]
    fn in_force_gating_is_status_driven() {
        let catalog = SectorCatalog::new();
        assert!(catalog.is_in_force("battery"));
        assert!(catalog.is_in_force("textile-unsold"));
        assert!(catalog.is_in_force("electronics"));
        assert!(!catalog.is_in_force("detergent")); // partial → flagged
        assert!(!catalog.is_in_force("nonexistent"));
    }

    #[test]
    fn allows_determination_matches_status() {
        assert!(RegulatoryStatus::InForce.allows_determination());
        assert!(!RegulatoryStatus::Provisional.allows_determination());
    }

    #[test]
    fn register_runtime_sector() {
        let mut catalog = SectorCatalog::new();
        let descriptor = SectorDescriptor {
            key: "plastics".into(),
            title: "Plastics".into(),
            status: RegulatoryStatus::Provisional,
            legal_basis: vec!["ESPR Working Plan".into()],
            dpp_applies_from: None,
            retention_years: 10,
            schema_versions: vec!["1.0.0".into()],
            current_schema_version: "1.0.0".into(),
            product_categories: vec![],
            access_tiers: std::collections::HashMap::new(),
            plugin: None,
            notes: None,
        };
        assert!(catalog.register(descriptor.clone()).is_ok());
        assert_eq!(catalog.len(), 12);
        assert!(matches!(
            catalog.register(descriptor),
            Err(CatalogError::AlreadyExists(_))
        ));
    }

    /// Parity guard: the closed [`Sector`] enum and the open [`SectorCatalog`]
    /// must describe the same set of *compile-time* sectors. Runtime-registered
    /// sectors degrade to `SectorData::Other`, but every typed `Sector` variant
    /// (except `Other`) must have an embedded catalog entry, and the embedded
    /// catalog must not carry a key with no corresponding variant. This stops
    /// the "four spellings of a sector" drift from reappearing across the
    /// enum ↔ catalog boundary.
    #[test]
    fn sector_enum_and_catalog_agree() {
        use crate::domain::sector::Sector;

        let catalog = SectorCatalog::new();

        // Every typed Sector variant (except Other) must be in the catalog.
        let typed = [
            Sector::Battery,
            Sector::Textile,
            Sector::TextileUnsoldGoods,
            Sector::Steel,
            Sector::Electronics,
            Sector::Construction,
            Sector::Tyre,
            Sector::Toy,
            Sector::Aluminium,
            Sector::Furniture,
            Sector::Detergent,
        ];
        for sector in &typed {
            let key = sector.catalog_key();
            assert!(
                catalog.get(key).is_some(),
                "Sector::{sector:?} (key '{key}') has no embedded catalog entry"
            );
        }

        // No catalog entry without a typed Sector variant.
        let typed_keys: std::collections::HashSet<&str> =
            typed.iter().map(Sector::catalog_key).collect();
        for key in catalog.keys() {
            assert!(
                typed_keys.contains(key),
                "catalog key '{key}' has no corresponding typed Sector variant"
            );
        }
    }

    /// Compliance citation (domain Gap / watchlist 🔴): the ESPR unsold-goods
    /// destruction ban is **Article 25 / Annex VII**, not Article 22. A wrong
    /// citation in a compliance artifact erodes auditor trust.
    /// Source: Regulation (EU) 2024/1781 (ESPR) Article 25, Annex VII.
    #[test]
    fn unsold_goods_cites_espr_article_25() {
        let catalog = SectorCatalog::new();
        let textile = catalog
            .get("textile-unsold")
            .expect("textile-unsold present");
        let basis = textile.legal_basis.join(" ");
        assert!(
            basis.contains("Article 25"),
            "unsold-goods legal basis must cite ESPR Article 25, got: {basis}"
        );
        assert!(
            !basis.contains("Article 22"),
            "the incorrect Article 22 citation must be gone, got: {basis}"
        );
    }

    #[test]
    fn descriptor_round_trips_camel_case() {
        let catalog = SectorCatalog::new();
        let battery = catalog.get("battery").unwrap();
        let json = serde_json::to_value(battery).unwrap();
        assert_eq!(json["dppAppliesFrom"], "2027-02-18");
        assert_eq!(json["status"], "in_force");
        let back: SectorDescriptor = serde_json::from_value(json).unwrap();
        assert_eq!(back.key, "battery");
    }

    // Drift guard: every key in a sector's access_tiers manifest must correspond to
    // a real JSON field in that sector's current schema. A key that doesn't match any
    // schema property silently fails to gate any field — the redaction is a no-op.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn access_tiers_keys_match_schema_properties() {
        use crate::schemas::VersionedSchemaRegistry;
        let catalog = SectorCatalog::new();
        let registry = VersionedSchemaRegistry::new();

        for descriptor in catalog.all() {
            if descriptor.access_tiers.is_empty() {
                continue;
            }
            let version: semver::Version = descriptor
                .current_schema_version
                .parse()
                .unwrap_or_else(|_| {
                    panic!(
                        "sector '{}' currentSchemaVersion '{}' is not valid semver",
                        descriptor.key, descriptor.current_schema_version
                    )
                });
            let schema_json = registry.get(&descriptor.key, &version).unwrap_or_else(|| {
                panic!(
                    "schema not found for sector '{}' v{}",
                    descriptor.key, descriptor.current_schema_version
                )
            });
            let schema: serde_json::Value =
                serde_json::from_str(schema_json).expect("embedded schema must be valid JSON");
            let properties = schema
                .get("properties")
                .and_then(|p| p.as_object())
                .unwrap_or_else(|| {
                    panic!(
                        "schema for sector '{}' has no top-level 'properties' object",
                        descriptor.key
                    )
                });

            for key in descriptor.access_tiers.keys() {
                assert!(
                    properties.contains_key(key),
                    "access_tiers key '{}' in sector '{}' does not match any property in schema v{} \
                     (properties: {:?}). Either rename the key to match the serialised field name, \
                     or remove it — a mismatched key silently fails to gate the field.",
                    key,
                    descriptor.key,
                    descriptor.current_schema_version,
                    properties.keys().collect::<Vec<_>>()
                );
            }
        }
    }

    // The key enforcement: catalog ↔ schema registry must agree, so the
    // "four spellings of a sector" problem cannot silently reappear.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn catalog_agrees_with_schema_registry() {
        use crate::schemas::VersionedSchemaRegistry;
        let catalog = SectorCatalog::new();
        let registry = VersionedSchemaRegistry::new();

        // Every schema version a sector declares must exist in the registry,
        // and its current version must be one of them.
        for d in catalog.all() {
            let reg_versions: Vec<String> = registry
                .versions_for(&d.key)
                .iter()
                .map(|v| v.to_string())
                .collect();
            for v in &d.schema_versions {
                assert!(
                    reg_versions.contains(v),
                    "catalog sector '{}' declares schema {v} not embedded in the registry (registry has {reg_versions:?})",
                    d.key
                );
            }
            assert!(
                d.schema_versions.contains(&d.current_schema_version),
                "catalog sector '{}' currentSchemaVersion {} is not in its schemaVersions {:?}",
                d.key,
                d.current_schema_version,
                d.schema_versions
            );
        }

        // No orphan schemas: every registry sector must have a catalog entry.
        for sector in registry.sectors() {
            assert!(
                catalog.get(sector).is_some(),
                "schema registry has sector '{sector}' with no catalog entry"
            );
        }

        // Every in-force sector must declare a plugin binding.
        for d in catalog.in_force() {
            assert!(
                d.plugin.is_some(),
                "in-force sector '{}' must declare a plugin binding",
                d.key
            );
        }
    }
}
