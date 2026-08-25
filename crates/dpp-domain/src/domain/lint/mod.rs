//! Passport plausibility lint dispatch — maps [`ProductGroupData`](crate::domain::product_group::ProductGroupData) onto the
//! `dpp-rules::lint` pack and carries the owned, serialisable wire types the
//! engine persists on [`crate::domain::passport::Passport::lint_result`].
//!
//! Unlike [`crate::ports::compliance`], there is no pluggable strategy here:
//! the lint pack ships directly in `dpp-rules` and is not an extension seam.

mod finding;
mod result;
#[cfg(test)]
mod tests;

pub use finding::{LintFinding, LintSeverity, lint_product_group_data};
pub use result::LintResult;
