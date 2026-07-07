//! [`VersionedSchemaRegistry`]: the compiled-schema cache and hot-reload store.
//!
//! Split out of `schemas::registry` (removed) so the type lives one level
//! shallower — `schemas::VersionedSchemaRegistry` — with `mod.rs` re-exporting
//! it as a pure index. See the crate CHANGELOG for the migration note.

use semver::Version;

use super::{SchemaEntry, SchemaOrigin, SchemaRegistrationError};

/// Thread-safe (via external `RwLock`) versioned JSON schema registry.
///
/// Starts pre-loaded with compile-time embedded schemas and accepts runtime
/// registrations for new sector versions or entirely new sectors.
///
/// # Hot-reload workflow
///
/// ```rust,ignore
/// let mut registry = VersionedSchemaRegistry::new();
///
/// // At startup: all embedded schemas are available
/// assert!(registry.get("battery", &"1.0.0".parse().unwrap()).is_some());
///
/// // At runtime: load a new schema version from disk / network
/// let new_schema = std::fs::read_to_string("schemas/battery/v3.0.0.json")?;
/// registry.register("battery", "3.0.0", new_schema)?;
///
/// // Now latest("battery") returns v3.0.0
/// let (ver, _) = registry.latest("battery").unwrap();
/// assert_eq!(*ver, "3.0.0".parse::<semver::Version>().unwrap());
/// ```
pub struct VersionedSchemaRegistry {
    entries: Vec<SchemaEntry>,
    /// Lazily-populated cache of compiled schemas, keyed by `(sector, version)`.
    /// Interior mutability keeps [`Self::validate`] `&self` and thread-safe; the
    /// `&mut self` mutators evict the affected key.
    #[cfg(not(target_arch = "wasm32"))]
    compiled: std::sync::RwLock<
        std::collections::HashMap<(String, Version), std::sync::Arc<jsonschema::JSONSchema>>,
    >,
}

impl VersionedSchemaRegistry {
    /// Create a new registry pre-loaded with all compile-time embedded schemas.
    #[must_use]
    pub fn new() -> Self {
        let entries = super::embedded::initial_entries();
        Self {
            entries,
            #[cfg(not(target_arch = "wasm32"))]
            compiled: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Validate that `json` is not just valid JSON but a compilable JSON Schema.
    ///
    /// On `wasm32` (no `jsonschema`), only the JSON parse is checked.
    fn check_compilable(json: &str) -> Result<(), SchemaRegistrationError> {
        let value: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| SchemaRegistrationError::InvalidJson(e.to_string()))?;
        #[cfg(not(target_arch = "wasm32"))]
        jsonschema::JSONSchema::compile(&value).map_err(|e| {
            SchemaRegistrationError::InvalidJson(format!("schema does not compile: {e}"))
        })?;
        #[cfg(target_arch = "wasm32")]
        let _ = value;
        Ok(())
    }

    /// Drop the cached compiled schema for a `(sector, version)`.
    #[cfg(not(target_arch = "wasm32"))]
    fn evict(&mut self, sector: &str, version: &Version) {
        self.compiled
            .get_mut()
            .expect("schema cache not poisoned")
            .remove(&(sector.to_owned(), version.clone()));
    }
    #[cfg(target_arch = "wasm32")]
    fn evict(&mut self, _sector: &str, _version: &Version) {}

    /// Register a new schema at runtime.
    ///
    /// Returns `Err(AlreadyExists)` if a schema for this (sector, version)
    /// already exists. Use [`Self::register_or_replace`] to overwrite.
    pub fn register(
        &mut self,
        sector: &str,
        version_str: &str,
        json: String,
    ) -> Result<(), SchemaRegistrationError> {
        let version: Version = version_str
            .parse()
            .map_err(|_| SchemaRegistrationError::InvalidVersion(version_str.to_owned()))?;

        // Reject anything that is not a compilable JSON Schema.
        Self::check_compilable(&json)?;

        if self
            .entries
            .iter()
            .any(|e| e.sector == sector && e.version == version)
        {
            return Err(SchemaRegistrationError::AlreadyExists {
                sector: sector.to_owned(),
                version,
            });
        }

        self.entries.push(SchemaEntry {
            sector: sector.to_owned(),
            version,
            json,
            origin: SchemaOrigin::Runtime,
        });

        Ok(())
    }

    /// Register a schema, replacing any existing entry for the same (sector, version).
    ///
    /// Returns `true` if an existing entry was replaced, `false` if this is new.
    pub fn register_or_replace(
        &mut self,
        sector: &str,
        version_str: &str,
        json: String,
    ) -> Result<bool, SchemaRegistrationError> {
        let version: Version = version_str
            .parse()
            .map_err(|_| SchemaRegistrationError::InvalidVersion(version_str.to_owned()))?;

        Self::check_compilable(&json)?;

        let replaced = if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|e| e.sector == sector && e.version == version)
        {
            existing.json = json;
            existing.origin = SchemaOrigin::Runtime;
            true
        } else {
            self.entries.push(SchemaEntry {
                sector: sector.to_owned(),
                version: version.clone(),
                json,
                origin: SchemaOrigin::Runtime,
            });
            false
        };

        // A replaced schema invalidates its compiled cache entry.
        if replaced {
            self.evict(sector, &version);
        }

        Ok(replaced)
    }

    /// Remove a runtime-registered schema.
    ///
    /// Returns `true` if the entry was found and removed. Embedded schemas
    /// cannot be removed (returns `false`).
    pub fn unregister(&mut self, sector: &str, version: &Version) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| {
            !(e.sector == sector && e.version == *version && e.origin == SchemaOrigin::Runtime)
        });
        let removed = self.entries.len() < before;
        if removed {
            self.evict(sector, version);
        }
        removed
    }

    /// Get the schema JSON for a sector at a specific version.
    pub fn get(&self, sector: &str, version: &Version) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| e.sector == sector && e.version == *version)
            .map(|e| e.json.as_str())
    }

    /// Get the full schema entry (including origin) for a sector at a specific version.
    pub fn get_entry(&self, sector: &str, version: &Version) -> Option<&SchemaEntry> {
        self.entries
            .iter()
            .find(|e| e.sector == sector && e.version == *version)
    }

    /// Get the latest schema for a sector.
    pub fn latest(&self, sector: &str) -> Option<(&Version, &str)> {
        self.entries
            .iter()
            .filter(|e| e.sector == sector)
            .max_by(|a, b| a.version.cmp(&b.version))
            .map(|e| (&e.version, e.json.as_str()))
    }

    /// List all available (sector, version) pairs.
    pub fn list(&self) -> Vec<(&str, &Version)> {
        self.entries
            .iter()
            .map(|e| (e.sector.as_str(), &e.version))
            .collect()
    }

    /// List all versions for a specific sector, sorted ascending.
    pub fn versions_for(&self, sector: &str) -> Vec<&Version> {
        let mut versions: Vec<&Version> = self
            .entries
            .iter()
            .filter(|e| e.sector == sector)
            .map(|e| &e.version)
            .collect();
        versions.sort();
        versions
    }

    /// List all unique sector names.
    pub fn sectors(&self) -> Vec<&str> {
        let mut sectors: Vec<&str> = self.entries.iter().map(|e| e.sector.as_str()).collect();
        sectors.sort();
        sectors.dedup();
        sectors
    }

    /// Count total schema entries (embedded + runtime).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the registry has no schemas.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Validate `data` against the schema for `(sector, version)`, failing closed
    /// when the version string is unparseable or no schema is registered for that
    /// sector/version pair.
    ///
    /// Use this on write paths where silently skipping validation is not acceptable.
    /// For optional validation that skips when no schema is registered, use
    /// [`Self::validate_if_present`].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn validate_strict(
        &self,
        sector: &str,
        version: &str,
        data: &serde_json::Value,
    ) -> Result<(), crate::domain::field_error::ValidationErrors> {
        use crate::domain::field_error::{FieldError, ValidationErrors};

        let version = version.parse::<Version>().map_err(|_| ValidationErrors {
            errors: vec![FieldError {
                field: "/schema_version".to_owned(),
                message: format!("schema version '{version}' is not a valid semver string"),
            }],
        })?;
        self.validate(sector, &version, data)
    }

    /// Validate against the schema for `sector` at the given version *string*,
    /// **skipping** (returning `Ok`) when the version is unparseable or no such
    /// schema is registered.
    ///
    /// This is the write-path convenience: callers enforce a JSON schema only
    /// when one actually exists for the sector/version, without depending on
    /// `semver` themselves. Sectors with no embedded schema fall through to the
    /// caller's typed validation.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn validate_if_present(
        &self,
        sector: &str,
        version: &str,
        data: &serde_json::Value,
    ) -> Result<(), crate::domain::field_error::ValidationErrors> {
        let Ok(version) = version.parse::<Version>() else {
            return Ok(());
        };
        if self.get(sector, &version).is_none() {
            return Ok(());
        }
        self.validate(sector, &version, data)
    }

    /// Validate data against the schema for the given sector and version.
    ///
    /// The compiled schema is cached per `(sector, version)` on first use, so
    /// repeated validations do not recompile. `&self` and thread-safe.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn validate(
        &self,
        sector: &str,
        version: &Version,
        data: &serde_json::Value,
    ) -> Result<(), crate::domain::field_error::ValidationErrors> {
        use crate::domain::field_error::{FieldError, ValidationErrors};

        let compiled = self
            .compiled_schema(sector, version)
            .ok_or_else(|| ValidationErrors {
                errors: vec![FieldError {
                    field: "/".to_owned(),
                    message: format!("no schema found for sector '{sector}' version '{version}'"),
                }],
            })?;

        // Edition 2024 drops the trailing-expression temporary (the borrowing
        // error iterator) before `compiled`, so the match can be returned directly.
        match compiled.validate(data) {
            Ok(()) => Ok(()),
            Err(errors) => Err(ValidationErrors {
                errors: errors
                    .map(|e| FieldError {
                        field: e.instance_path.to_string(),
                        message: e.to_string(),
                    })
                    .collect(),
            }),
        }
    }

    /// Get the compiled schema for `(sector, version)`, compiling and caching it
    /// on first use. Returns `None` if no such schema is registered.
    #[cfg(not(target_arch = "wasm32"))]
    fn compiled_schema(
        &self,
        sector: &str,
        version: &Version,
    ) -> Option<std::sync::Arc<jsonschema::JSONSchema>> {
        let key = (sector.to_owned(), version.clone());
        if let Some(cached) = self
            .compiled
            .read()
            .expect("schema cache not poisoned")
            .get(&key)
        {
            return Some(cached.clone());
        }
        let json = self.get(sector, version)?;
        // Embedded schemas are valid JSON by construction; runtime schemas are
        // compile-checked at registration. Use ok()? instead of expect() so
        // a future broken schema returns None (→ "no schema found" error) rather
        // than panicking the process.
        let value = serde_json::from_str::<serde_json::Value>(json).ok()?;
        let compiled = std::sync::Arc::new(jsonschema::JSONSchema::compile(&value).ok()?);
        self.compiled
            .write()
            .expect("schema cache not poisoned")
            .insert(key, compiled.clone());
        Some(compiled)
    }
}

impl Default for VersionedSchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}
