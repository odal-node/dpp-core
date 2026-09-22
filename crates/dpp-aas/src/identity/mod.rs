//! What the AAS shell says the asset is.
//!
//! One type, in its own file per rule 2: a `mod.rs` is an index.

mod asset_identity;

pub use asset_identity::{AssetIdentity, RESERVED_ASSET_ID_NAMES};
