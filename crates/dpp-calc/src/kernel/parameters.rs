//! The numbers a ruleset carries, and the rule that governs replacing them.
//!
//! A signed bundle may **fill** a ruleset's parameters and may never
//! **override** ones that come from law. The test for which is which is
//! [`ParameterBasis`] — provenance, not presence. Every ruleset in this crate
//! already *has* numbers; only some of them are the Official Journal's, and a
//! rule keyed on "does a value exist" either freezes every placeholder forever
//! or lets a bundle contradict an act.
//!
//! # Identity stays compile-time
//!
//! A bundle delivers *parameters*, never a ruleset. [`RulesetId`] and
//! [`RulesetVersion`](super::ruleset::RulesetVersion) remain `&'static str`
//! literals, so nothing here can introduce a ruleset that was not compiled in —
//! a new delegated act still needs a release. That is the deliberate limit of
//! this channel, not an oversight: see the `Ruleset` identity types, whose docs
//! record the same choice.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use dpp_rules::bundle::{RulesetAcceptance, RulesetProvenance};

use super::error::CalcError;
use super::hashing::jcs_hash;
use super::ruleset::{ParameterBasis, Ruleset, RulesetId};

/// Top-level key in a bundle's `content` under which per-ruleset parameter
/// slices are keyed by [`RulesetId`].
const CONTENT_RULESETS_KEY: &str = "rulesets";

/// The numbers a ruleset carries, as named groups of canonical JSON.
///
/// Groups rather than a flat map of scalars on purpose. A set like
/// [`RepairabilityWeights`](crate::repairability::thresholds::RepairabilityWeights) has an
/// invariant across its members — they must sum to 1.0 — and letting a bundle
/// replace one weight without the others would break that invariant silently.
/// Replacing a whole group is the smallest edit that can still be validated as
/// a unit.
///
/// Serialises transparently as a JSON object, so the JCS hash in
/// [`content_sha256`](Self::content_sha256) is the hash of exactly what a reader
/// would see.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RulesetParameters(BTreeMap<String, serde_json::Value>);

impl RulesetParameters {
    /// An empty parameter set.
    #[must_use]
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Add a named group, serialising `value` to JSON.
    ///
    /// The builder every `Ruleset::parameters()` implementation uses, so a
    /// ruleset declares its numbers once rather than restating them in a second
    /// shape that can drift from the statics it computes with.
    pub fn with(
        mut self,
        name: impl Into<String>,
        value: &impl Serialize,
    ) -> Result<Self, CalcError> {
        let json =
            serde_json::to_value(value).map_err(|e| CalcError::CanonicalizeError(e.to_string()))?;
        self.0.insert(name.into(), json);
        Ok(self)
    }

    /// The named group, if present.
    #[must_use]
    pub fn group(&self, name: &str) -> Option<&serde_json::Value> {
        self.0.get(name)
    }

    /// Deserialise a named group into its typed form.
    ///
    /// Fails if the group is missing or does not match `T`. Types deserialised
    /// from a bundle should carry `#[serde(deny_unknown_fields)]`, or a
    /// misspelled key arrives as a silent no-op and the operator believes a
    /// threshold changed when it did not.
    pub fn typed_group<T: serde::de::DeserializeOwned>(&self, name: &str) -> Result<T, CalcError> {
        let raw = self.0.get(name).ok_or_else(|| {
            CalcError::InvalidInput(format!("parameter group '{name}' is not present"))
        })?;
        serde_json::from_value(raw.clone())
            .map_err(|e| CalcError::InvalidInput(format!("parameter group '{name}': {e}")))
    }

    /// Group names, in canonical order.
    pub fn group_names(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    /// Whether any group is present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Hex SHA-256 over the JCS (RFC 8785) canonical JSON of these parameters.
    ///
    /// This is what makes a determination reproducible rather than merely
    /// attributed. `ruleset_id` and `ruleset_version` say *which rule*; without
    /// this, two receipts citing the same id and version but computed by builds
    /// carrying different numbers are indistinguishable.
    pub fn content_sha256(&self) -> Result<String, CalcError> {
        jcs_hash(self)
    }
}

/// Which signed bundle delivered a filled parameter set.
///
/// Both fields travel together because either alone is weak: `bundle_version`
/// is a label the publisher chose and can reuse, and a content hash without a
/// version does not say which release an auditor should go and fetch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleProvenance {
    /// The bundle's `bundleVersion`, e.g. `"2026-Q3.1"`.
    pub bundle_version: String,
    /// The manifest's `contentSha256` — the bytes the publisher signed over.
    pub content_sha256: String,
}

/// The parameter slice a **verified** bundle offers for `ruleset_id`, with the
/// provenance to stamp on any receipt computed from it.
///
/// `Ok(None)` when the bundle carries nothing for this ruleset, which is the
/// ordinary case for most rulesets in any given bundle — not an error, and the
/// caller stays on its compiled-in numbers.
///
/// # Why an unverified acceptance is refused
///
/// [`RulesetProvenance::LocalBaseline`] exists because a node with no configured
/// channel has to start somewhere, and it seeds itself from compiled-in defaults
/// that have no signature to check. Reading parameters *out* of such an
/// acceptance is a different act from starting up with it: it would let bytes
/// that nothing authenticated replace a ruleset's numbers. So the fill path
/// takes only [`RulesetProvenance::Verified`], and this is the fail-closed
/// direction the bundle format was built for.
pub fn offered_for(
    acceptance: &RulesetAcceptance,
    ruleset_id: &RulesetId,
) -> Result<Option<(RulesetParameters, BundleProvenance)>, CalcError> {
    if acceptance.provenance() != RulesetProvenance::Verified {
        return Err(CalcError::UnverifiedBundle {
            ruleset_id: ruleset_id.0.to_owned(),
        });
    }

    // A bundle carrying no `rulesets` key at all offers nothing here, which is
    // ordinary. One carrying a `rulesets` key that is not an object is a
    // publisher mistake, and `Value::get` would answer `None` to it exactly as
    // it does to the ordinary case — so the bundle would appear to apply cleanly
    // while every slice inside it was skipped. Named instead.
    let rulesets = match acceptance.content().get(CONTENT_RULESETS_KEY) {
        None => return Ok(None),
        Some(serde_json::Value::Object(map)) => map,
        Some(other) => {
            return Err(CalcError::InvalidInput(format!(
                "bundle '{}' has a '{CONTENT_RULESETS_KEY}' that is {}, not an object — no \
                 ruleset in it would be applied",
                acceptance.version(),
                json_kind(other)
            )));
        }
    };

    let Some(slice) = rulesets.get(ruleset_id.0) else {
        return Ok(None);
    };

    let offered: RulesetParameters = serde_json::from_value(slice.clone()).map_err(|e| {
        CalcError::InvalidInput(format!(
            "bundle parameters for ruleset '{}' are malformed: {e}",
            ruleset_id.0
        ))
    })?;

    let provenance = BundleProvenance {
        bundle_version: acceptance.version().to_owned(),
        content_sha256: acceptance.manifest().content_sha256.clone(),
    };

    Ok(Some((offered, provenance)))
}

/// Apply `offered` to `ruleset`'s compiled-in parameters — fill, never override.
///
/// # The rule
///
/// - A ruleset whose [`ParameterBasis`] is
///   [`Sourced`](ParameterBasis::Sourced) is refused outright. Its numbers are
///   the instrument's, and a bundle that could rewrite them could make a node
///   report a legal threshold that is not the law.
/// - A ruleset that is [`Assumed`](ParameterBasis::Assumed) accepts a group at a
///   time, and only groups it already declares.
///
/// # Why unknown groups are refused rather than ignored
///
/// Serde's default for an unrecognised key is to drop it. Applied here that
/// would mean a bundle naming `"weight"` instead of `"weights"` changes nothing
/// and reports success — the operator believes a threshold moved, every receipt
/// says the parameters are unchanged, and the two are consistent with each
/// other. Refusing names the key instead.
///
/// A type change is refused for the same reason: a group offered as a string
/// where the ruleset holds an object is a mistake worth naming at the fill,
/// rather than a deserialisation failure further along with no bundle in the
/// message.
pub fn fill<R: Ruleset + ?Sized>(
    ruleset: &R,
    offered: &RulesetParameters,
) -> Result<RulesetParameters, CalcError> {
    if ruleset.parameter_basis() == ParameterBasis::Sourced {
        return Err(CalcError::SourcedParametersNotFillable {
            ruleset_id: ruleset.id().0.to_owned(),
        });
    }

    let mut filled = ruleset.parameters();

    for (name, value) in &offered.0 {
        let Some(existing) = filled.0.get(name) else {
            return Err(CalcError::UnknownParameterGroup {
                ruleset_id: ruleset.id().0.to_owned(),
                name: name.clone(),
                known: filled.group_names().collect::<Vec<_>>().join(", "),
            });
        };
        if json_kind(existing) != json_kind(value) {
            return Err(CalcError::ParameterGroupTypeMismatch {
                ruleset_id: ruleset.id().0.to_owned(),
                name: name.clone(),
                expected: json_kind(existing),
                got: json_kind(value),
            });
        }
        filled.0.insert(name.clone(), value.clone());
    }

    Ok(filled)
}

/// The JSON type name of `value`, for type-mismatch messages.
fn json_kind(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}
