//! `FactorProvider` trait for injecting licensed LCI emission factor datasets at runtime.

use super::error::CalcError;

// Test/feature-only synthetic provider, kept in its own file so production code
// never compiles it unless explicitly opted in. Re-exported here to preserve the
// `factor::SyntheticFactorProvider` path.
#[cfg(any(test, feature = "synthetic-factors"))]
pub use super::synthetic_factor::SyntheticFactorProvider;

/// Provides lifecycle-inventory emission factors at runtime.
///
/// Implementations inject licensed datasets (ecoinvent, EF, Sphera) without
/// bundling the underlying data in this Apache-2.0 crate — the methodology is
/// open, the factor data is licensed and supplied at runtime.
///
/// A `FactorProvider` is intentionally *not* used by the Phase-0 calculators
/// (repairability and the basic CO₂e cradle-to-gate), which take emission
/// factors as caller-supplied inputs. It will be wired in Phase 2 when the
/// battery CFB engine requires real LCI data.
pub trait FactorProvider: Send + Sync {
    /// Machine-readable identifier of the loaded dataset (e.g. `"ecoinvent-3.10"`).
    fn dataset_id(&self) -> &str;
    /// Version string of the loaded dataset.
    fn dataset_version(&self) -> &str;
    /// GWP100 characterization factor for an activity identified by its
    /// dataset-internal UUID, in kg CO₂e per kg of substance emitted.
    fn gwp100(&self, activity_uuid: &str) -> Result<f64, CalcError>;
    /// SHA-256 of the full serialised factor table, computed once at provider
    /// initialisation. Stored in the [`CalculationReceipt`] so a notified body
    /// can verify that the exact factor values are unchanged between calculations.
    ///
    /// [`CalculationReceipt`]: super::receipt::CalculationReceipt
    fn table_hash(&self) -> &str;
}
