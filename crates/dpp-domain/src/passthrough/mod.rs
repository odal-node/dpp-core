//! Open-core compliance boundary: the Apache-2.0 passthrough registry.
//!
//! The canonical compliance seam is the [`ComplianceRegistry`] /
//! [`ComplianceStrategy`](crate::ports::compliance::ComplianceStrategy) pair in
//! [`ports::compliance`](crate::ports::compliance); [`PassthroughRegistry`]
//! is the open-source default implementation wired by the OSS binary.
//!
//! [`PassthroughRegistry`] dispatches **through** the strategy trait rather than
//! around it: [`strategies`] holds the two Apache-2.0 strategies it
//! registers, and any product group without one takes a bare-passthrough fallback. That
//! keeps the per-product group seam exercised by the default build instead of being a
//! trait nothing implements.
//!
//! [`ComplianceRegistry`]: crate::ports::compliance::ComplianceRegistry
//! [`PassthroughRegistry`]: registry::PassthroughRegistry

pub mod registry;
pub mod strategies;

pub use registry::PassthroughRegistry;
pub use strategies::{PassthroughBatteryStrategy, PassthroughTextileStrategy};

#[cfg(test)]
mod registry_tests;
#[cfg(test)]
mod strategies_tests;
