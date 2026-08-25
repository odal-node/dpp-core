//! Open-core compliance boundary: the Apache-2.0 passthrough registry.
//!
//! The canonical compliance seam is the [`ComplianceRegistry`] /
//! [`ComplianceStrategy`](crate::ports::compliance::ComplianceStrategy) pair in
//! [`ports::compliance`](crate::ports::compliance); [`PassthroughRegistry`]
//! is the open-source default implementation wired by the OSS binary.
//!
//! [`PassthroughRegistry`] dispatches **through** the strategy trait rather than
//! around it: [`passthrough_strategies`] holds the two Apache-2.0 strategies it
//! registers, and any product group without one takes a bare-passthrough fallback. That
//! keeps the per-product group seam exercised by the default build instead of being a
//! trait nothing implements.
//!
//! [`ComplianceRegistry`]: crate::ports::compliance::ComplianceRegistry
//! [`PassthroughRegistry`]: passthrough_registry::PassthroughRegistry

pub mod passthrough_registry;
pub mod passthrough_strategies;

pub use passthrough_registry::PassthroughRegistry;
pub use passthrough_strategies::{PassthroughBatteryStrategy, PassthroughTextileStrategy};

#[cfg(test)]
mod passthrough_registry_tests;
#[cfg(test)]
mod passthrough_strategies_tests;
