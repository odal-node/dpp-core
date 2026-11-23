//! Helpers more than one module of this crate uses.
//!
//! Each sits here because two siblings need it, not because it is general: a
//! helper one module needs lives in that module.

pub(crate) mod jwt;
pub(crate) mod media_type;
