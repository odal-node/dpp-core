//! Open-source passthrough compliance registry.
//!
//! Returns manufacturer-supplied data verbatim for **every** sector, computing
//! nothing. A *determination* (computed metrics, pass/fail) is the job of the
//! Wasm sector plugins (the canonical OSS determination path) or a proprietary
//! tier.
//!
//! This is the Apache-2.0 default. The [`ComplianceRegistry`] /
//! [`ComplianceStrategy`] traits remain the extension seam a proprietary tier
//! wires its own implementation into — and this registry now **dispatches
//! through** the strategy trait rather than around it, so the seam is exercised
//! by the default build.

use std::collections::HashMap;

use super::passthrough_strategies::{PassthroughBatteryStrategy, PassthroughTextileStrategy};
use crate::{
    domain::sector::SectorData,
    ports::compliance::{
        ComplianceError, ComplianceRegistry, ComplianceResult, ComplianceStrategy,
    },
};

/// Open-source passthrough compliance registry.
///
/// Makes no determination for any sector and computes no metrics: every sector
/// yields
/// [`ComplianceStatus::PassthroughNoValidation`](crate::ports::compliance::ComplianceStatus::PassthroughNoValidation).
///
/// # Dispatch
///
/// Sectors with a registered [`ComplianceStrategy`] are routed to it; every
/// other sector falls back to a bare
/// [`ComplianceResult::passthrough`]. Both paths produce the same *status* — the
/// difference is that a strategy lifts that sector's declared metrics into the
/// result's sector-agnostic fields, and the fallback has no way to know which
/// field of an arbitrary payload is a CO₂e figure.
///
/// **An unregistered sector is not an error.** The catalog is open by design —
/// a product group can be added as manifest plus schema, with no Rust — so
/// `UnknownSector` here would make the registry the one closed part of an
/// otherwise data-driven model. It is reserved for a registry that genuinely
/// cannot serve a sector, which this one always can.
pub struct PassthroughRegistry {
    strategies: HashMap<String, Box<dyn ComplianceStrategy>>,
}

impl PassthroughRegistry {
    /// Build the registry with the Apache-2.0 strategies registered.
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            strategies: HashMap::new(),
        };
        registry.register(Box::new(PassthroughBatteryStrategy));
        registry.register(Box::new(PassthroughTextileStrategy));
        registry
    }

    /// Build a registry with no strategies at all — every sector takes the
    /// bare-passthrough fallback.
    ///
    /// For a host that wants the fallback behaviour and intends to register its
    /// own strategies, without first having to displace these.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            strategies: HashMap::new(),
        }
    }

    /// Register `strategy` under the key it reports, replacing any strategy
    /// already registered for that sector.
    ///
    /// Replacing rather than refusing is deliberate: this is the documented way
    /// a proprietary tier substitutes one sector's behaviour without
    /// reimplementing the registry, and a silent refusal would leave it running
    /// the passthrough while believing otherwise.
    pub fn register(&mut self, strategy: Box<dyn ComplianceStrategy>) {
        self.strategies
            .insert(strategy.sector_key().to_owned(), strategy);
    }

    /// The catalog keys this registry has a strategy for, sorted.
    #[must_use]
    pub fn registered_sectors(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = self.strategies.keys().map(String::as_str).collect();
        keys.sort_unstable();
        keys
    }
}

impl Default for PassthroughRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ComplianceRegistry for PassthroughRegistry {
    fn compute(
        &self,
        sector_key: &str,
        data: &SectorData,
    ) -> Result<ComplianceResult, ComplianceError> {
        match self.strategies.get(sector_key) {
            Some(strategy) => strategy.compute(data),
            None => Ok(ComplianceResult::passthrough()),
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::sector::{BatteryData, FibreEntry, Sector, SectorData, TextileData};
    use crate::ports::compliance::ComplianceStatus;

    fn battery_data() -> SectorData {
        SectorData::Battery(Box::new(BatteryData {
            recycled_content_lithium_pct: Some(12.5),
            rated_capacity_kwh: Some(32.0),
            ..crate::test_support::sample_battery_data()
        }))
    }

    fn textile_data() -> SectorData {
        SectorData::Textile(Box::new(TextileData {
            fibre_composition: vec![FibreEntry {
                fibre: "cotton".into(),
                pct: 100.0,
                country_of_origin: None,
            }],
            country_of_origin: "BD".into(),
            care_instructions: "40°C wash".into(),
            chemical_compliance_standard: "OEKO-TEX 100".into(),
            recycled_content_pct: Some(30.0),
            carbon_footprint_kg_co2e: Some(8.5),
            repair_score: Some(7.5),
            ..crate::test_support::sample_textile_data()
        }))
    }

    /// No sector gets a determination, whether or not it has a strategy.
    ///
    /// "Determination" is the *status* and the findings — that is what a
    /// notified body reads and what blocks a publish. Metrics are not a
    /// determination: `ComplianceResult::co2e_score` is documented as
    /// "calculated **or manufacturer-supplied**", and carrying a declared value
    /// under `PassthroughNoValidation` claims nothing about it.
    ///
    /// This test previously also asserted the metrics were `None`, which was
    /// true of the stub rather than of passthrough. Lifting a declared metric is
    /// exactly what "stores manufacturer-supplied values verbatim" means; the
    /// invariant that must not move is the one asserted here.
    #[test]
    fn passthrough_makes_no_determination_for_any_sector() {
        let registry = PassthroughRegistry::new();
        for (sector, data) in [
            (Sector::Battery, battery_data()),
            (Sector::Textile, textile_data()),
            // A sector with no per-sector handling used to return NotImplemented;
            // now it takes the bare-passthrough fallback.
            (Sector::Electronics, battery_data()),
        ] {
            let result = registry.compute(sector.catalog_key(), &data).unwrap();
            assert_eq!(
                result.compliance_status,
                ComplianceStatus::PassthroughNoValidation,
                "{sector:?} must not receive a determination"
            );
            assert!(
                result.violations.is_empty() && result.warnings.is_empty(),
                "{sector:?} passthrough must produce no findings"
            );
            assert!(
                result.receipt.is_none() && result.ruleset_version.is_none(),
                "{sector:?} ran no calculation, so it has no receipt to show for one"
            );
        }
    }

    /// A registered sector routes through its strategy; an unregistered one does
    /// not error.
    ///
    /// The catalog is open by design — a product group can be added as manifest
    /// plus schema with no Rust — so a sector without a strategy must still be
    /// served. `UnknownSector` here would make this registry the one closed part
    /// of a data-driven model.
    #[test]
    fn a_registered_sector_uses_its_strategy_and_the_rest_fall_back() {
        let registry = PassthroughRegistry::new();
        assert_eq!(registry.registered_sectors(), vec!["battery", "textile"]);

        // Textile has a strategy: its declared metrics are lifted.
        let textile = registry.compute("textile", &textile_data()).unwrap();
        assert_eq!(textile.co2e_score, Some(8.5));
        assert_eq!(textile.recycled_content_pct, Some(30.0));
        assert_eq!(textile.repairability_index, Some(7.5));

        // Electronics has none: served, with nothing lifted.
        let electronics = registry.compute("electronics", &battery_data()).unwrap();
        assert_eq!(electronics.co2e_score, None);
        assert_eq!(
            electronics.compliance_status,
            ComplianceStatus::PassthroughNoValidation
        );

        // A sector this build has never heard of is served too.
        let unknown = registry.compute("quantum-widget", &battery_data());
        assert!(
            unknown.is_ok(),
            "an unmodelled sector must not be an error here"
        );
    }

    /// `register` replaces, so a tier can substitute one sector's behaviour.
    ///
    /// This is the whole point of the per-sector seam. A silent refusal would
    /// leave the host running passthrough while believing it had swapped in its
    /// own strategy.
    #[test]
    fn registering_a_strategy_replaces_the_one_already_there() {
        struct AlwaysFortyTwo;
        impl ComplianceStrategy for AlwaysFortyTwo {
            fn sector_key(&self) -> &str {
                "textile"
            }
            fn compute(&self, _: &SectorData) -> Result<ComplianceResult, ComplianceError> {
                Ok(ComplianceResult {
                    co2e_score: Some(42.0),
                    ..ComplianceResult::passthrough()
                })
            }
        }

        let mut registry = PassthroughRegistry::new();
        registry.register(Box::new(AlwaysFortyTwo));
        assert_eq!(
            registry
                .compute("textile", &textile_data())
                .unwrap()
                .co2e_score,
            Some(42.0),
            "the registered strategy must displace the built-in one"
        );
        assert_eq!(
            registry.registered_sectors(),
            vec!["battery", "textile"],
            "replacing must not add a second entry for the same sector"
        );
    }

    /// `empty()` registers nothing, so every sector takes the fallback.
    #[test]
    fn an_empty_registry_falls_back_for_everything() {
        let registry = PassthroughRegistry::empty();
        assert!(registry.registered_sectors().is_empty());
        let result = registry.compute("textile", &textile_data()).unwrap();
        assert_eq!(result.co2e_score, None);
        assert_eq!(
            result.compliance_status,
            ComplianceStatus::PassthroughNoValidation
        );
    }
}
