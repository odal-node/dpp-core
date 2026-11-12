//! Reading the dated schedule a verified signed ruleset bundle carries.
//!
//! # Where it sits in a bundle
//!
//! A bundle's `content` is a JSON object of named sections, each read by the
//! consumer that owns it. The schedule is the `obligations` section: a list of
//! entries shaped as an [`ObligationEvent`] is on the wire, without its `bundle`
//! field. That field is filled here, from the manifest the bundle was verified
//! against, so an entry cannot claim to come from anywhere else.
//!
//! A bundle carries the whole schedule, not a change to the previous one: the
//! bundle in force replaces its predecessor entirely, so an entry left out of a
//! newer bundle is no longer announced.

use std::collections::BTreeSet;

use chrono::NaiveDate;
use dpp_rules::bundle::{RulesetAcceptance, RulesetProvenance};
use serde::Deserialize;

use super::{BundleOrigin, ObligationEvent, ObligationKind, ScheduleBundleError};
use crate::instrument::DateBasis;

/// Top-level key in a bundle's `content` under which the schedule sits.
const CONTENT_OBLIGATIONS_KEY: &str = "obligations";

/// One schedule entry as a bundle carries it.
///
/// `deny_unknown_fields`, or a misspelled key would arrive as a silent default
/// and an operator would be told a date for the wrong product group or
/// categories.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BundledObligation {
    id: String,
    date: NaiveDate,
    kind: ObligationKind,
    citation: String,
    summary: String,
    #[serde(default)]
    product_group: Option<String>,
    #[serde(default)]
    categories: Vec<String>,
    basis: DateBasis,
}

/// The schedule `acceptance` carries, as events stamped with the bundle they
/// came from, and that bundle.
///
/// No events when the bundle has no `obligations` section, which is ordinary: a
/// bundle may carry only other content.
pub(super) fn read(
    acceptance: &RulesetAcceptance,
) -> Result<(BundleOrigin, Vec<ObligationEvent>), ScheduleBundleError> {
    let bundle_version = acceptance.version().to_owned();
    if acceptance.provenance() != RulesetProvenance::Verified {
        return Err(ScheduleBundleError::Unverified { bundle_version });
    }
    let origin = BundleOrigin {
        bundle_version,
        content_sha256: acceptance.manifest().content_sha256.clone(),
    };
    let malformed = |reason: String| ScheduleBundleError::Malformed {
        bundle_version: origin.bundle_version.clone(),
        reason,
    };

    let Some(section) = acceptance.content().get(CONTENT_OBLIGATIONS_KEY) else {
        return Ok((origin, Vec::new()));
    };
    let entries = Vec::<BundledObligation>::deserialize(section)
        .map_err(|e| malformed(format!("'{CONTENT_OBLIGATIONS_KEY}': {e}")))?;

    let mut seen = BTreeSet::new();
    if let Some(repeated) = entries.iter().find(|e| !seen.insert(e.id.as_str())) {
        return Err(malformed(format!(
            "the id '{}' appears more than once",
            repeated.id
        )));
    }

    let events = entries
        .into_iter()
        .map(|e| ObligationEvent {
            id: e.id,
            date: e.date,
            kind: e.kind,
            citation: e.citation,
            summary: e.summary,
            product_group: e.product_group,
            categories: e.categories,
            basis: e.basis,
            bundle: Some(origin.clone()),
        })
        .collect();
    Ok((origin, events))
}
