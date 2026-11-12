//! Passport plausibility lint dispatch — maps [`ProductGroupData`](crate::product_group::ProductGroupData) onto the
//! `dpp-rules::lint` pack and carries the owned, serialisable wire types the
//! engine persists on [`crate::passport::Passport::lint_result`].
//!
//! Unlike [`crate::ports::compliance`], there is no pluggable strategy here:
//! the lint pack ships directly in `dpp-rules` and is not an extension seam.

mod finding;
mod result;
#[cfg(test)]
mod tests;

pub use finding::{
    LintFinding, LintSeverity, lint_discard_record, lint_product_group_data,
    lint_unsold_goods_report,
};
pub use result::LintResult;
