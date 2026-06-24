//! Open-core compliance boundary: the Apache-2.0 passthrough registry.
//!
//! The canonical compliance seam is the [`ComplianceRegistry`] /
//! [`ComplianceStrategy`](crate::ports::compliance::ComplianceStrategy) pair in
//! [`ports::compliance`](crate::ports::compliance); [`PassthroughRegistry`]
//! is the open-source default implementation wired by the OSS binary.
//!
//! [`ComplianceRegistry`]: crate::ports::compliance::ComplianceRegistry
//! [`PassthroughRegistry`]: passthrough_registry::PassthroughRegistry

pub mod passthrough_registry;
