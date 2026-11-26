//! What the SD-JWT oracle's corpus modules share.
//!
//! - [`base`] is the small credential the adversarial cases mutate, and the helpers
//!   that build one case.
//! - [`plan`] issues a claim tree to a plan and works out what any presentation of
//!   it must reveal.
//! - [`signing`] is the issuer key the corpus is signed with, how a token is
//!   assembled, and the verdict this crate gives one.
//! - [`challenge`] is the Verifier the bound cases are for.

mod base;
mod challenge;
mod plan;
mod signing;

pub(super) use base::*;
pub(super) use challenge::*;
pub(super) use plan::*;
pub(super) use signing::*;
