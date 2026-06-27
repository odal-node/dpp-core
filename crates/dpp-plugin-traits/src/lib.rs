//! Host/guest ABI contract for Odal Node sector plugins.
//!
//! Plugins implement [`DppSectorPlugin`] and export the three entry points
//! as `extern "C"` symbols. The host invokes them through the wasmtime
//! component model or directly via the low-level ABI defined below.
//!
//! The interface is intentionally `no_std`-friendly: no heap allocations
//! are required from the host's perspective. Data is passed as JSON strings
//! over a shared-memory slice.
//!
//! ## Versioning
//!
//! Every plugin declares which ABI version and schema versions it supports
//! via [`PluginCapabilities`]. The host uses this for compatibility checks
//! before dispatching any calls.
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ─── ABI version ────────────────────────────────────────────────────────────

/// Current host ABI version.
///
/// Increment the major version for breaking changes to the plugin interface.
/// Increment the minor version for backward-compatible additions.
pub const ABI_VERSION_MAJOR: u32 = 1;
// 1.1: PluginResult gained backward-compatible `violations`/`warnings` finding
// lists. Older (1.0) plugins omit them (serde defaults to empty) and still load.
pub const ABI_VERSION_MINOR: u32 = 1;

/// ABI version declared by a plugin.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AbiVersion {
    pub major: u32,
    pub minor: u32,
}

impl AbiVersion {
    pub const fn current() -> Self {
        Self {
            major: ABI_VERSION_MAJOR,
            minor: ABI_VERSION_MINOR,
        }
    }

    /// Check if this ABI version is compatible with the host.
    ///
    /// Major versions must match exactly. The plugin's minor version must be
    /// ≤ the host's minor version (the host supports all older minor versions).
    #[allow(clippy::absurd_extreme_comparisons)] // intentional: works correctly when ABI_VERSION_MINOR > 0
    pub fn is_compatible_with_host(&self) -> bool {
        self.major == ABI_VERSION_MAJOR && self.minor <= ABI_VERSION_MINOR
    }
}

impl std::fmt::Display for AbiVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

// ─── Plugin identity ────────────────────────────────────────────────────────

/// Static metadata returned by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMeta {
    /// Sector key this plugin handles, e.g. `"textile"`, `"steel"`, `"battery"`.
    pub sector: String,
    /// Human-readable plugin name.
    pub name: String,
    /// SemVer version string of the plugin itself.
    pub version: String,
    /// SPDX license identifier.
    pub license: String,
    /// Brief description of what this plugin does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Plugin author or organisation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// URL for plugin documentation or source code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
}

// ─── Capabilities ───────────────────────────────────────────────────────────

/// Schema version range a plugin supports.
///
/// A plugin may support multiple schema versions (e.g., it can validate
/// both v1.0.0 and v1.1.0 textile data). The host uses this to dispatch
/// data to the correct plugin version.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SchemaVersionRange {
    /// Minimum supported schema version (inclusive), e.g. `"1.0.0"`.
    pub min_version: String,
    /// Maximum supported schema version (inclusive), e.g. `"1.1.0"`.
    pub max_version: String,
}

/// Feature flags a plugin may declare support for.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    /// Can validate sector-specific data against the schema.
    Validate,
    /// Can compute compliance metrics (CO2e, repairability, etc.).
    ComputeMetrics,
    /// Can generate a passport-ready data payload.
    GeneratePassport,
    /// Can perform SVHC / substance-of-concern screening.
    SubstanceScreening,
    /// Can compute lifecycle assessment (LCA) metrics.
    LifecycleAssessment,
    /// Can map data to Asset Administration Shell (AAS) submodels.
    AasMapping,
    /// Custom capability (plugin-defined extension point).
    Custom(String),
}

/// Full capability declaration returned by a plugin during negotiation.
///
/// The host calls `capabilities()` before dispatching any work to verify
/// that the plugin supports the required schema version and features.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCapabilities {
    /// The ABI version this plugin was compiled against.
    pub abi_version: AbiVersion,
    /// The sector schemas this plugin can handle.
    pub supported_schemas: Vec<SchemaVersionRange>,
    /// Feature capabilities this plugin provides.
    pub capabilities: Vec<PluginCapability>,
    /// Minimum host ABI version required by this plugin.
    /// If the host's ABI is below this, the plugin refuses to load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_host_version: Option<AbiVersion>,
    /// Plugin-declared fuel budget per invocation (host caps at DEFAULT_FUEL).
    /// Plugins needing less computation can set this lower for tighter sandboxing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fuel: Option<u64>,
    /// Plugin-declared memory cap in bytes per invocation (host caps at DEFAULT_MEMORY_CAP_BYTES).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_memory_bytes: Option<u64>,
}

// ─── Compatibility check result ─────────────────────────────────────────────

/// Result of a compatibility check between host and plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompatibilityStatus {
    /// Fully compatible — all checks pass.
    Compatible,
    /// ABI version mismatch — major version differs.
    AbiIncompatible {
        host: AbiVersion,
        plugin: AbiVersion,
    },
    /// Plugin requires a newer host than what's running.
    HostTooOld {
        required: AbiVersion,
        actual: AbiVersion,
    },
    /// The plugin doesn't support the requested schema version.
    SchemaUnsupported {
        requested: String,
        supported: Vec<SchemaVersionRange>,
    },
    /// Missing a required capability.
    MissingCapability(PluginCapability),
}

impl CompatibilityStatus {
    pub fn is_compatible(&self) -> bool {
        matches!(self, Self::Compatible)
    }
}

/// Check if a plugin is compatible with the current host and a requested
/// schema version.
pub fn check_compatibility(
    capabilities: &PluginCapabilities,
    requested_schema_version: Option<&str>,
    required_capabilities: &[PluginCapability],
) -> CompatibilityStatus {
    // 1. ABI version check
    if !capabilities.abi_version.is_compatible_with_host() {
        return CompatibilityStatus::AbiIncompatible {
            host: AbiVersion::current(),
            plugin: capabilities.abi_version,
        };
    }

    // 2. Min host version check
    if let Some(ref min_host) = capabilities.min_host_version {
        let current = AbiVersion::current();
        if current.major < min_host.major
            || (current.major == min_host.major && current.minor < min_host.minor)
        {
            return CompatibilityStatus::HostTooOld {
                required: *min_host,
                actual: current,
            };
        }
    }

    // 3. Schema version check (semantic via semver crate; falls back to lexicographic)
    if let Some(requested) = requested_schema_version {
        let req = semver::Version::parse(requested).ok();
        let supported = capabilities.supported_schemas.iter().any(|range| {
            let lo = semver::Version::parse(&range.min_version).ok();
            let hi = semver::Version::parse(&range.max_version).ok();
            match (req.as_ref(), lo, hi) {
                (Some(r), Some(l), Some(h)) => r >= &l && r <= &h,
                _ => {
                    requested >= range.min_version.as_str()
                        && requested <= range.max_version.as_str()
                }
            }
        });
        if !supported {
            return CompatibilityStatus::SchemaUnsupported {
                requested: requested.to_owned(),
                supported: capabilities.supported_schemas.clone(),
            };
        }
    }

    // 4. Capability check
    for required in required_capabilities {
        if !capabilities.capabilities.contains(required) {
            return CompatibilityStatus::MissingCapability(required.clone());
        }
    }

    CompatibilityStatus::Compatible
}

// ─── Plugin input/output ────────────────────────────────────────────────────

/// Raw JSON input passed from the host to a plugin entry point.
pub type PluginInput = serde_json::Value;

/// Typed compliance determination returned by a plugin.
///
/// Mirrors `dpp_domain::ports::compliance::ComplianceStatus` so the host maps
/// directly from one typed enum to the other rather than parsing a string.
/// Serialised as SCREAMING_SNAKE_CASE JSON strings for ABI stability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PluginComplianceStatus {
    Compliant,
    NonCompliant,
    NotAssessed,
    PassthroughNoValidation,
    NotImplemented,
}

/// Well-known metric key for CO₂e score (kg CO₂e per functional unit).
pub const METRIC_CO2E_SCORE: &str = "co2e_score";
/// Well-known metric key for EN 45554 repairability index (0.0–10.0).
pub const METRIC_REPAIRABILITY_INDEX: &str = "repairability_index";
/// Well-known metric key for recycled content percentage (0.0–100.0).
pub const METRIC_RECYCLED_CONTENT_PCT: &str = "recycled_content_pct";

/// A single determination finding emitted by a plugin's `calculate_metrics`.
///
/// Findings split into [`PluginResult::violations`] (binding — the host blocks
/// publish for an in-force sector) and [`PluginResult::warnings`]
/// (advisory/experimental — surfaced, never blocks). The vec encodes severity,
/// so there is no separate severity field. Maps 1:1 onto the host's
/// `ComplianceFinding`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginFinding {
    /// Stable machine-readable code, e.g. `"battery.recycled_content.cobalt_below_2031"`.
    pub code: String,
    /// JSON-pointer-style field locator (e.g. `"/recycledContentCobaltPct"`), or
    /// empty when the finding is not tied to a single field.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub field: String,
    /// Human-readable explanation.
    pub message: String,
}

impl PluginFinding {
    /// Construct a finding from its code, field locator, and message.
    pub fn new(
        code: impl Into<String>,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            field: field.into(),
            message: message.into(),
        }
    }
}

/// Compliance result returned by the plugin.
///
/// `metrics` is a sector-extensible map of named numeric values. Use the
/// `METRIC_*` constants for the three well-known fields; plugins may add
/// sector-specific keys (`"water_use_litres"`, `"pef_score"`, …).
///
/// ## Aligning with `ComplianceResult` on the host
///
/// ```text
/// co2e_score           ← metrics["co2e_score"]
/// repairability_index  ← metrics["repairability_index"]
/// recycled_content_pct ← metrics["recycled_content_pct"]
/// compliance_status    ← PluginComplianceStatus → ComplianceStatus (typed map)
/// violations/warnings  ← PluginFinding → ComplianceFinding (host blocks on violations)
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginResult {
    /// Typed compliance determination.
    pub compliance_status: PluginComplianceStatus,
    /// Sector-extensible keyed metric map (all values finite f64).
    #[serde(default)]
    pub metrics: std::collections::HashMap<String, f64>,
    /// Non-numeric sector-specific data (free-form; stored verbatim in extra).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
    /// Binding findings — the host blocks publish for an in-force sector. Empty
    /// for passthrough / not-assessed determinations. (ABI 1.1+)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub violations: Vec<PluginFinding>,
    /// Advisory / experimental findings — surfaced but never block publish (e.g.
    /// recycled-content thresholds that are not yet in force). (ABI 1.1+)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<PluginFinding>,
}

impl PluginResult {
    /// Construct a result carrying only a compliance status.
    pub fn new(status: PluginComplianceStatus) -> Self {
        Self {
            compliance_status: status,
            metrics: std::collections::HashMap::new(),
            extra: None,
            violations: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Insert a metric unconditionally (non-finite values are silently dropped).
    pub fn with_metric(mut self, key: &str, value: f64) -> Self {
        if value.is_finite() {
            self.metrics.insert(key.to_owned(), value);
        }
        self
    }

    /// Insert a metric only when `value` is `Some` (non-finite values dropped).
    pub fn maybe_metric(mut self, key: &str, value: Option<f64>) -> Self {
        if let Some(v) = value
            && v.is_finite()
        {
            self.metrics.insert(key.to_owned(), v);
        }
        self
    }

    /// Attach free-form non-numeric extra data.
    pub fn with_extra(mut self, extra: serde_json::Value) -> Self {
        self.extra = Some(extra);
        self
    }

    /// Append a binding violation (host blocks publish for an in-force sector).
    pub fn with_violation(mut self, finding: PluginFinding) -> Self {
        self.violations.push(finding);
        self
    }

    /// Append an advisory warning (surfaced, never blocks publish).
    pub fn with_warning(mut self, finding: PluginFinding) -> Self {
        self.warnings.push(finding);
        self
    }

    // ── Convenience accessors for the three well-known metrics ──────────────

    pub fn co2e_score(&self) -> Option<f64> {
        self.metrics.get(METRIC_CO2E_SCORE).copied()
    }

    pub fn repairability_index(&self) -> Option<f64> {
        self.metrics.get(METRIC_REPAIRABILITY_INDEX).copied()
    }

    pub fn recycled_content_pct(&self) -> Option<f64> {
        self.metrics.get(METRIC_RECYCLED_CONTENT_PCT).copied()
    }
}

// ─── ABI response envelope ────────────────────────────────────────────────────

/// JSON envelope wrapping the outcome of a fallible plugin ABI call.
///
/// The low-level Wasm exports (`validate`, `calculate_metrics`,
/// `generate_passport`) cannot return a Rust `Result` across the C ABI, so the
/// outcome is serialised as this externally-tagged enum. On success, `ok`
/// carries the method's return value (a [`PluginResult`] for
/// `calculate_metrics`, the normalised payload for `generate_passport`, or
/// `null` for `validate`). On failure, `error` carries a structured
/// [`PluginError`]. The host deserialises this to recover the typed result.
///
/// ```json
/// { "ok": { "co2eScore": 85.4, "complianceStatus": "NOT_ASSESSED", ... } }
/// { "error": { "ValidationErrors": [ { "field": "/gtin", "code": "missing", ... } ] } }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbiResult {
    Ok(serde_json::Value),
    Error(PluginError),
}

impl AbiResult {
    /// Build a successful response by serialising `value`.
    pub fn ok<T: Serialize>(value: &T) -> Self {
        Self::Ok(serde_json::to_value(value).unwrap_or(serde_json::Value::Null))
    }

    /// Returns `true` if this is the success variant.
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }
}

// ─── Plugin errors ──────────────────────────────────────────────────────────

/// Structured error with field-level detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginFieldError {
    /// JSON pointer to the failing field, e.g. `"/fibreComposition/0/pct"`.
    pub field: String,
    /// Error code for programmatic handling (e.g. `"out_of_range"`, `"missing"`).
    pub code: String,
    /// Human-readable error message.
    pub message: String,
}

#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum PluginError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("validation errors: {0:?}")]
    ValidationErrors(Vec<PluginFieldError>),
    #[error("calculation failed: {0}")]
    Calculation(String),
    #[error("sector not supported by this plugin: {0}")]
    UnsupportedSector(String),
    #[error("schema version not supported: {0}")]
    UnsupportedSchemaVersion(String),
    #[error("capability not available: {0}")]
    CapabilityNotAvailable(String),
    #[error("internal plugin error: {0}")]
    Internal(String),
}

// ─── Host-side trait ────────────────────────────────────────────────────────

/// The entry points every sector plugin must export.
///
/// The Wasm host calls these after deserialising JSON sector data from the
/// passport payload. Implementations must be deterministic and free of I/O.
pub trait DppSectorPlugin: Send + Sync {
    /// Returns static metadata about this plugin.
    fn meta(&self) -> PluginMeta;

    /// Returns the plugin's capability declaration for version negotiation.
    fn capabilities(&self) -> PluginCapabilities;

    /// Validate the structure and field constraints of the sector input.
    ///
    /// Returns `Ok(())` if the input is structurally valid, or a descriptive
    /// error if a required field is missing or out of range. Prefer
    /// `PluginError::ValidationErrors` with per-field detail over
    /// `PluginError::InvalidInput` for better error reporting.
    fn validate_input(&self, input: &PluginInput) -> Result<(), PluginError>;

    /// Compute compliance metrics from the sector input.
    ///
    /// May return `None` for fields that do not apply to this sector.
    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError>;

    /// Generate a passport-ready sector data JSON payload.
    ///
    /// Applies any normalisation or enrichment required by the sector schema
    /// (e.g. rounding, unit conversion). The output is stored verbatim in the DPP.
    fn generate_passport(&self, input: &PluginInput) -> Result<serde_json::Value, PluginError>;
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_capabilities() -> PluginCapabilities {
        PluginCapabilities {
            abi_version: AbiVersion::current(),
            supported_schemas: vec![SchemaVersionRange {
                min_version: "1.0.0".into(),
                max_version: "1.1.0".into(),
            }],
            capabilities: vec![
                PluginCapability::Validate,
                PluginCapability::ComputeMetrics,
                PluginCapability::GeneratePassport,
            ],
            min_host_version: None,
            max_fuel: None,
            max_memory_bytes: None,
        }
    }

    #[test]
    fn abi_version_current_is_compatible() {
        let current = AbiVersion::current();
        assert!(current.is_compatible_with_host());
    }

    #[test]
    fn abi_version_major_mismatch_incompatible() {
        let future = AbiVersion { major: 2, minor: 0 };
        assert!(!future.is_compatible_with_host());
    }

    #[test]
    fn abi_version_minor_ahead_incompatible() {
        let ahead = AbiVersion {
            major: ABI_VERSION_MAJOR,
            minor: ABI_VERSION_MINOR + 1,
        };
        assert!(!ahead.is_compatible_with_host());
    }

    #[test]
    fn abi_version_display() {
        let v = AbiVersion { major: 1, minor: 0 };
        assert_eq!(format!("{v}"), "1.0");
    }

    #[test]
    fn compatibility_check_passes() {
        let caps = sample_capabilities();
        let result = check_compatibility(&caps, Some("1.0.0"), &[PluginCapability::Validate]);
        assert!(result.is_compatible());
    }

    #[test]
    fn compatibility_check_schema_in_range() {
        let caps = sample_capabilities();
        let result = check_compatibility(&caps, Some("1.1.0"), &[]);
        assert!(result.is_compatible());
    }

    #[test]
    fn compatibility_check_schema_out_of_range() {
        let caps = sample_capabilities();
        let result = check_compatibility(&caps, Some("2.0.0"), &[]);
        assert!(matches!(
            result,
            CompatibilityStatus::SchemaUnsupported { .. }
        ));
    }

    #[test]
    fn semver_multi_digit_minor_accepted() {
        // Lexicographic comparison would reject "1.10.0" within ["1.0.0", "1.10.0"]
        // because "1.10.0" < "1.2.0" as strings. Semantic comparison must handle this.
        let caps = PluginCapabilities {
            abi_version: AbiVersion::current(),
            supported_schemas: vec![SchemaVersionRange {
                min_version: "1.0.0".into(),
                max_version: "1.10.0".into(),
            }],
            capabilities: vec![],
            min_host_version: None,
            max_fuel: None,
            max_memory_bytes: None,
        };
        let result = check_compatibility(&caps, Some("1.10.0"), &[]);
        assert!(
            result.is_compatible(),
            "1.10.0 must be accepted within [1.0.0, 1.10.0]"
        );
    }

    #[test]
    fn semver_multi_digit_minor_rejected_correctly() {
        // "1.10.0" must be rejected when max is "1.2.0"
        let caps = PluginCapabilities {
            abi_version: AbiVersion::current(),
            supported_schemas: vec![SchemaVersionRange {
                min_version: "1.0.0".into(),
                max_version: "1.2.0".into(),
            }],
            capabilities: vec![],
            min_host_version: None,
            max_fuel: None,
            max_memory_bytes: None,
        };
        let result = check_compatibility(&caps, Some("1.10.0"), &[]);
        assert!(
            matches!(result, CompatibilityStatus::SchemaUnsupported { .. }),
            "1.10.0 must be rejected when max is 1.2.0"
        );
    }

    #[test]
    fn compatibility_check_missing_capability() {
        let caps = sample_capabilities();
        let result = check_compatibility(&caps, None, &[PluginCapability::SubstanceScreening]);
        assert!(matches!(result, CompatibilityStatus::MissingCapability(_)));
    }

    #[test]
    fn compatibility_check_abi_mismatch() {
        let mut caps = sample_capabilities();
        caps.abi_version = AbiVersion { major: 2, minor: 0 };
        let result = check_compatibility(&caps, None, &[]);
        assert!(matches!(
            result,
            CompatibilityStatus::AbiIncompatible { .. }
        ));
    }

    #[test]
    fn compatibility_check_host_too_old() {
        let mut caps = sample_capabilities();
        caps.min_host_version = Some(AbiVersion {
            major: ABI_VERSION_MAJOR,
            minor: ABI_VERSION_MINOR + 5,
        });
        let result = check_compatibility(&caps, None, &[]);
        assert!(matches!(result, CompatibilityStatus::HostTooOld { .. }));
    }

    #[test]
    fn compatibility_check_no_schema_constraint() {
        let caps = sample_capabilities();
        let result = check_compatibility(&caps, None, &[]);
        assert!(result.is_compatible());
    }

    #[test]
    fn plugin_meta_round_trip() {
        let meta = PluginMeta {
            sector: "textile".into(),
            name: "Textile Compliance Plugin".into(),
            version: "0.2.0".into(),
            license: "Apache-2.0".into(),
            description: Some("Validates textile DPP data".into()),
            author: Some("Odal Node".into()),
            homepage: Some("https://github.com/odal-node".into()),
        };
        let json = serde_json::to_value(&meta).unwrap();
        assert_eq!(json["sector"], "textile");
        assert_eq!(json["description"], "Validates textile DPP data");
        let back: PluginMeta = serde_json::from_value(json).unwrap();
        assert_eq!(meta.name, back.name);
    }

    #[test]
    fn capabilities_round_trip() {
        let caps = sample_capabilities();
        let json = serde_json::to_value(&caps).unwrap();
        assert!(json["supportedSchemas"].is_array());
        assert_eq!(json["abiVersion"]["major"], ABI_VERSION_MAJOR);
        let back: PluginCapabilities = serde_json::from_value(json).unwrap();
        assert_eq!(caps.abi_version, back.abi_version);
    }

    #[test]
    fn plugin_field_error_round_trip() {
        let err = PluginFieldError {
            field: "/fibreComposition/0/pct".into(),
            code: "out_of_range".into(),
            message: "pct must be 0-100".into(),
        };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["code"], "out_of_range");
        let back: PluginFieldError = serde_json::from_value(json).unwrap();
        assert_eq!(err.field, back.field);
    }

    #[test]
    fn custom_capability_round_trip() {
        let cap = PluginCapability::Custom("carbon_offset_calc".into());
        let json = serde_json::to_value(&cap).unwrap();
        let back: PluginCapability = serde_json::from_value(json).unwrap();
        assert_eq!(cap, back);
    }

    #[test]
    fn abi_result_ok_round_trip() {
        let result = PluginResult::new(PluginComplianceStatus::NotAssessed)
            .with_metric(METRIC_CO2E_SCORE, 85.4)
            .with_metric(METRIC_RECYCLED_CONTENT_PCT, 12.5);
        let envelope = AbiResult::ok(&result);
        assert!(envelope.is_ok());
        let json = serde_json::to_value(&envelope).unwrap();
        assert!(json["ok"].is_object());
        assert_eq!(json["ok"]["complianceStatus"], "NOT_ASSESSED");

        let back: AbiResult = serde_json::from_value(json).unwrap();
        match back {
            AbiResult::Ok(v) => assert_eq!(v["metrics"]["co2e_score"], 85.4),
            AbiResult::Error(_) => panic!("expected ok variant"),
        }
    }

    #[test]
    fn abi_result_error_round_trip() {
        let envelope = AbiResult::Error(PluginError::ValidationErrors(vec![PluginFieldError {
            field: "/gtin".into(),
            code: "missing".into(),
            message: "gtin is required".into(),
        }]));
        assert!(!envelope.is_ok());
        let json = serde_json::to_value(&envelope).unwrap();
        assert!(json.get("error").is_some());

        let back: AbiResult = serde_json::from_value(json).unwrap();
        assert!(!back.is_ok());
    }
}
