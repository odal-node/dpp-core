//! A regulatory claim in schema prose must name what it rests on.
//!
//! Every other gate step in this repo reads Rust. Nothing read a `description`
//! string, and every recorded instance of a fabricated regulatory claim in this
//! project has been in prose or data rather than in code — including two
//! electronics descriptions that once asserted an adoption date, an effective
//! date and a phase-two date **for an act that does not exist**, and shipped to
//! crates.io.
//!
//! # What the audit found, and why these are the rules
//!
//! Reading all 659 descriptions produced one clean split. Every defect was
//! either a bare assertion with no citation — five product groups asserting a
//! DPP mandate date, four checkably wrong — or a citation that did not hold: a
//! battery field citing *Annex XIII point 1(q)*, which enumerates the marking
//! requirements of Art. 13(3) and (4) and does not reach Art. 13(5). Every
//! description carrying a checkable citation checked out, without exception.
//!
//! So the rules below are not stylistic. They encode the two shapes that
//! actually went wrong.
//!
//! # What this can and cannot prove
//!
//! It cannot check that a citation is **true** — that needs the OJ text and a
//! reader. It checks one is present, well-formed, and points at an act this
//! crate knows, turning the next audit from research into a lookup. A green run
//! is not a statement that the prose is correct.
//!
//! # These were verified to fail
//!
//! Each rule was checked by editing `steel/v1.0.0`'s root description and
//! re-running. `per Annex IV point 3, applies from 2032.` fires Rules A and D;
//! `DPP mandate 2031. Regulation (EU) 2027/9999 applies from 2032.` fires B
//! and C. The second deliberately does *not* trip A or D — naming an act
//! anchors both, and Rule B is what asks whether the act exists.
//!
//! # What it found on the day it was written
//!
//! Twelve things, in prose that had already survived a full manual audit: six
//! descriptions asserting a passport date (`detergent` ×2, `toy` ×2, `furniture`
//! v1.2.0, `mattress` v1.0.0), six citing REACH with no act number, two citing
//! repealed acts nowhere recorded, and the toy schema calling Regulation (EU)
//! 2025/2509 a *Delegated* Regulation when the OJ header reads "REGULATION (EU)
//! 2025/2509 OF THE EUROPEAN PARLIAMENT AND OF THE COUNCIL".

use std::collections::BTreeSet;

use super::prose_act_reference_tests::{CITED_NOT_MODELLED, act_refs, cites_article_or_annex};
use crate::instrument::InstrumentCatalog;
use crate::schemas::VersionedSchemaRegistry;

/// **Rule A — an article or annex citation is anchored to an act.**
///
/// A description saying *"Annex XIII point 1(q)"* is checkable only if the
/// reader knows which act's Annex XIII. The corpus already works this way:
/// battery's root description names *EU Battery Regulation 2023/1542*, and its
/// ~100 field descriptions then cite articles and annexes of it. This makes that
/// convention a rule rather than a habit.
///
/// The anchor is the **root** description specifically, not any description in
/// the file. That distinction is not pedantry — it was wrong the first way
/// round. Furniture's *"SVHC substances above 0.1% w/w per REACH Article 33"*
/// counted as anchored because a completely unrelated field said a repairability
/// score is *"not EN 45554 / EU 2023/1669"*. An act named in passing, in a
/// disclaimer, about another field, anchors nothing.
///
/// A field description may still carry its own act, which is what lets one
/// schema cite a second instrument — so the requirement is: the act is named in
/// the root description, or in the citing description itself.
#[test]
fn an_article_or_annex_citation_names_its_act() {
    let mut unanchored: Vec<String> = Vec::new();

    for schema in schema_prose() {
        let anchored_by_root = !act_refs(&schema.root_description).is_empty();
        if anchored_by_root {
            continue;
        }
        for description in &schema.descriptions {
            if cites_article_or_annex(description) && act_refs(description).is_empty() {
                unanchored.push(format!(
                    "{} v{}: cites an article or annex, and neither this \
                     description nor the schema's root description names an act \
                     by number — {}",
                    schema.product_group,
                    schema.version,
                    truncate(description)
                ));
            }
        }
    }

    assert!(
        unanchored.is_empty(),
        "{} citation(s) rest on an act nothing names. Name it by number, in the \
         root description or in the citing description:\n{}",
        unanchored.len(),
        unanchored.join("\n")
    );
}

/// **Rule B — every act cited in prose is one this crate knows.**
///
/// The defect that opened this thread was prose citing an act that does not
/// exist. An act number is now resolved to a CELEX identifier and matched
/// against the instrument catalog, or against [`CITED_NOT_MODELLED`] — which is
/// an inventory with reasons, not a suppression list.
#[test]
fn every_act_cited_in_prose_is_known() {
    let catalog = InstrumentCatalog::new();
    let modelled: BTreeSet<&str> = catalog
        .all()
        .iter()
        .filter_map(|i| i.celex.as_deref())
        .collect();
    let inventoried: BTreeSet<&str> = CITED_NOT_MODELLED.iter().map(|(celex, _)| *celex).collect();

    let mut unknown: Vec<String> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();

    for schema in schema_prose() {
        for description in &schema.descriptions {
            for act in act_refs(description) {
                if modelled.contains(act.celex.as_str()) || inventoried.contains(act.celex.as_str())
                {
                    seen.insert(act.celex);
                    continue;
                }
                unknown.push(format!(
                    "{} v{}: '{}' resolves to CELEX {} which is \
                     neither in the instrument catalog nor in CITED_NOT_MODELLED",
                    schema.product_group, schema.version, act.text, act.celex
                ));
            }
        }
    }

    assert!(
        unknown.is_empty(),
        "{} act reference(s) point at nothing this crate holds. Either model the \
         instrument or add it to CITED_NOT_MODELLED with a reason:\n{}",
        unknown.len(),
        unknown.join("\n")
    );

    // An inventory that outlives its citations is stale rather than harmless:
    // it is read as a list of things we cite.
    let stale: Vec<&str> = inventoried
        .iter()
        .filter(|celex| !seen.contains(**celex))
        .copied()
        .collect();
    assert!(
        stale.is_empty(),
        "CITED_NOT_MODELLED lists {} act(s) no schema cites any more — remove \
         them: {stale:?}",
        stale.len()
    );
}

/// **Rule C — schema prose does not state a passport applicability date.**
///
/// This is the rule the audit established, and it is a prohibition rather than a
/// citation requirement, because even a *correct* year would not license the
/// claim. The ESPR working plan's column is headed *"Indicative timeline for
/// adoption"* — when a delegated act is adopted, not when an obligation applies,
/// with a transition in between. A description saying "DPP mandate 2030" states
/// the second while sourcing the first.
///
/// The date belongs in `crates/dpp-domain/instruments/`, where
/// `passport.from { date, basis }` carries its provenance and an indicative
/// adoption year is recorded in the binding's notes as indicative. A schema
/// description has nowhere to put a `basis`, which is exactly why it is the
/// wrong home.
#[test]
fn schema_prose_states_no_passport_applicability_date() {
    const CLAIMS: &[&str] = &[
        "dpp mandate",
        "passport mandate",
        "dpp applies",
        "dpp requirements",
        "delegated-act adoption",
        "mandate expected",
        "mandate is expected",
    ];

    let mut asserted: Vec<String> = Vec::new();
    for schema in schema_prose() {
        for description in &schema.descriptions {
            let lowered = description.to_lowercase();
            for claim in CLAIMS {
                if lowered.contains(claim) {
                    asserted.push(format!(
                        "{} v{}: '{claim}' — {}",
                        schema.product_group,
                        schema.version,
                        truncate(description)
                    ));
                }
            }
        }
    }

    assert!(
        asserted.is_empty(),
        "{} description(s) assert a passport applicability date. That belongs in \
         the instrument catalog, where it carries a basis:\n{}",
        asserted.len(),
        asserted.join("\n")
    );
}

/// **Rule D — any other date claim names an act in the same description.**
///
/// Rule C removes passport dates outright. An act's *own* application date is a
/// different thing and is legitimate prose — electronics says Regulations (EU)
/// 2023/1670 and 2023/1669 are "both applying from 20 June 2025", and tyre says
/// Regulation 2020/740 is "effective 1 May 2021". Both are true and both name
/// the act they are about.
///
/// So this requires proximity rather than mere presence: a date claim must name
/// its act in the **same** description, not merely somewhere in the file. A
/// reader checking "applies from X" needs to know what applies.
#[test]
fn a_date_claim_names_its_act_in_the_same_description() {
    const DATE_CLAIMS: &[&str] = &[
        "applies from",
        "applying from",
        "applicable from",
        "in force since",
        "entered into force",
        "effective ",
        "phased",
    ];

    let mut unanchored: Vec<String> = Vec::new();
    for schema in schema_prose() {
        for description in &schema.descriptions {
            let lowered = description.to_lowercase();
            let makes_claim = DATE_CLAIMS.iter().any(|c| lowered.contains(c));
            if makes_claim && act_refs(description).is_empty() {
                unanchored.push(format!(
                    "{} v{}: {}",
                    schema.product_group,
                    schema.version,
                    truncate(description)
                ));
            }
        }
    }

    assert!(
        unanchored.is_empty(),
        "{} date claim(s) name no act. State which instrument applies from that \
         date:\n{}",
        unanchored.len(),
        unanchored.join("\n")
    );
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// One schema's prose: what the file says about itself, and everything in it.
struct SchemaProse {
    product_group: String,
    version: String,
    /// The document-level `description`. The schema's statement of what
    /// instrument it implements, and the only anchor Rule A accepts.
    root_description: String,
    /// Every `description` in the file, the root one included.
    descriptions: Vec<String>,
}

/// Every schema in the registry, with its prose.
fn schema_prose() -> Vec<SchemaProse> {
    let registry = VersionedSchemaRegistry::new();
    let mut out = Vec::new();

    for product_group in registry.product_groups() {
        for version in registry.versions_for(product_group) {
            let json_text = registry
                .get(product_group, version)
                .expect("the registry listed it");
            let schema: serde_json::Value =
                serde_json::from_str(json_text).expect("schema is valid JSON");
            let mut descriptions = Vec::new();
            collect_descriptions(&schema, &mut descriptions);
            out.push(SchemaProse {
                product_group: product_group.to_owned(),
                version: version.to_string(),
                root_description: schema
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or_default()
                    .to_owned(),
                descriptions,
            });
        }
    }

    assert!(!out.is_empty(), "no schemas were read at all");
    out
}

/// Every `description` string anywhere in a schema document.
fn collect_descriptions(node: &serde_json::Value, out: &mut Vec<String>) {
    match node {
        serde_json::Value::Object(map) => {
            if let Some(description) = map.get("description").and_then(|d| d.as_str()) {
                out.push(description.to_owned());
            }
            for value in map.values() {
                collect_descriptions(value, out);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_descriptions(item, out);
            }
        }
        _ => {}
    }
}

/// Enough of a description to identify it in a failure message.
fn truncate(description: &str) -> String {
    let cut = description
        .char_indices()
        .nth(120)
        .map_or(description.len(), |(i, _)| i);
    if cut == description.len() {
        description.to_owned()
    } else {
        format!("{}…", &description[..cut])
    }
}
