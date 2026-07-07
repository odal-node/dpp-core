//! Port traits defining the core/platform boundary — one port per infrastructure concern.

pub mod archive;
pub mod compliance;
mod ghosts;
pub mod identity_port;
pub mod passport_repo;
pub mod plugin_host_port;
pub mod registry_sync;
pub mod seal;
