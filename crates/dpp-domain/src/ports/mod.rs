//! Port traits defining the core/platform boundary — one port per infrastructure concern.

pub mod archive;
pub mod compliance;
mod ghosts;
pub mod identity_port;
pub mod passport_repo;
#[cfg(test)]
mod passport_repo_tests;
pub mod plugin_host_port;
#[cfg(test)]
mod protected_patch_fields_tests;
pub mod registry_sync;
pub mod seal;
