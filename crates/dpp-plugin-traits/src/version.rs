//! ABI versioning and host/plugin compatibility negotiation.

use serde::{Deserialize, Serialize};

use crate::meta::{PluginCapabilities, PluginCapability};

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

    // 3. Schema version check — strictly semantic (semver). A version string
    // that isn't valid semver cannot be compared as a version; treat such a
    // range as non-matching rather than falling back to a lexicographic string
    // comparison (where e.g. "1.9.0" > "1.10.0" gives the wrong answer).
    if let Some(requested) = requested_schema_version {
        let req = semver::Version::parse(requested).ok();
        let supported = capabilities.supported_schemas.iter().any(|range| {
            match (
                req.as_ref(),
                semver::Version::parse(&range.min_version),
                semver::Version::parse(&range.max_version),
            ) {
                (Some(r), Ok(l), Ok(h)) => r >= &l && r <= &h,
                _ => false,
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
