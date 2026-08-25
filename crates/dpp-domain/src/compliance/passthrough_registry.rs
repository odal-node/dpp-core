//! Open-source passthrough compliance registry.
//!
//! Returns manufacturer-supplied data verbatim for **every** product group, computing
//! nothing. A *determination* (computed metrics, pass/fail) is the job of the
//! Wasm product group plugins (the canonical OSS determination path) or a proprietary
//! tier.
//!
//! This is the Apache-2.0 default. The [`ComplianceRegistry`] /
//! [`ComplianceStrategy`] traits remain the extension seam a proprietary tier
//! wires its own implementation into — and this registry now **dispatches
//! through** the strategy trait rather than around it, so the seam is exercised
//! by the default build.

use std::collections::HashMap;

use chrono::NaiveDate;

use super::passthrough_strategies::{PassthroughBatteryStrategy, PassthroughTextileStrategy};
use crate::{
    domain::product_group::ProductGroupData,
    ports::compliance::{
        ComplianceError, ComplianceRegistry, ComplianceResult, ComplianceStrategy,
    },
};

/// Open-source passthrough compliance registry.
///
/// Makes no determination for any product group and computes no metrics: every product group
/// yields
/// [`ComplianceStatus::PassthroughNoValidation`](crate::ports::compliance::ComplianceStatus::PassthroughNoValidation).
///
/// # Dispatch
///
/// ProductGroups with a registered [`ComplianceStrategy`] are routed to it; every
/// other product group falls back to a bare
/// [`ComplianceResult::passthrough`]. Both paths produce the same *status* — the
/// difference is that a strategy lifts that product group's declared metrics into the
/// result's product group-agnostic fields, and the fallback has no way to know which
/// field of an arbitrary payload is a CO₂e figure.
///
/// **An unregistered product group is not an error.** The catalog is open by design —
/// a product group can be added as manifest plus schema, with no Rust — so
/// `UnknownProductGroup` here would make the registry the one closed part of an
/// otherwise data-driven model. It is reserved for a registry that genuinely
/// cannot serve a product group, which this one always can.
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

    /// Build a registry with no strategies at all — every product group takes the
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
    /// already registered for that product group.
    ///
    /// Replacing rather than refusing is deliberate: this is the documented way
    /// a proprietary tier substitutes one product group's behaviour without
    /// reimplementing the registry, and a silent refusal would leave it running
    /// the passthrough while believing otherwise.
    pub fn register(&mut self, strategy: Box<dyn ComplianceStrategy>) {
        self.strategies
            .insert(strategy.product_group_key().to_owned(), strategy);
    }

    /// The catalog keys this registry has a strategy for, sorted.
    #[must_use]
    pub fn registered_product_groups(&self) -> Vec<&str> {
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
        product_group_key: &str,
        data: &ProductGroupData,
        law_in_force_on: Option<NaiveDate>,
    ) -> Result<ComplianceResult, ComplianceError> {
        match self.strategies.get(product_group_key) {
            Some(strategy) => strategy.compute(data, law_in_force_on),
            None => Ok(ComplianceResult::passthrough()),
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────
