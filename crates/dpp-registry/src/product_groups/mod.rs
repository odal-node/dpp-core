//! The registry's product groups, by the codes it names them with.
//!
//! The registry has its own vocabulary for a product group, separate from this
//! workspace's: its interface lists groups from
//! [`PRODUCT_GROUPS_PATH`](crate::PRODUCT_GROUPS_PATH), each with a `code`, and
//! a registration names its group by that code. A registration filed under this
//! workspace's own name for the group would name a group the registry does not
//! have.

mod code;
#[cfg(test)]
mod tests;

pub use code::{PRODUCT_GROUP_BATTERIES, registry_product_group};
