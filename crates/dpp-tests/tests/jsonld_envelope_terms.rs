//! Every member of the envelope's own objects has a JSON-LD term, at the position
//! it occupies.
//!
//! # What this holds the context to
//!
//! The manufacturer, the materials, the compliance result and the other envelope
//! objects each hold members of their own. A JSON-LD term is reachable only at the
//! position its key occupies, and a key with no term is dropped on expansion
//! together with everything inside it, with no error: the context defined the
//! envelope's keys and nothing beneath them, so a consumer of the linked-data form
//! saw `manufacturer` and not its name.
//!
//! Their terms are a table in `dpp_vc` (`envelope_terms`), because they are Rust
//! types and not schemas. This file serialises a passport whose every nested
//! object has every field set (`jsonld_support/envelope_instance.rs`, written as
//! struct literals so a new field does not compile until it is named) and holds
//! the table to what comes out, both ways. `productGroupData` is
//! `jsonld_product_group_terms.rs`.
//!
//! It asserts terms rather than running an expansion, because the workspace has
//! no JSON-LD processor. `jsonld-oracle.yml` expands the same passport with PyLD,
//! which is what says the terms are reachable and not merely present.

use std::collections::BTreeSet;

use dpp_domain::{Passport, ProductGroup, ProductGroupData};
use dpp_tests::fixtures::base_passport;
use dpp_vc::context_value;
use serde_json::{Map, Value, json};

#[path = "jsonld_support/envelope_instance.rs"]
mod envelope_instance;
#[path = "jsonld_support/lockstep.rs"]
mod lockstep;

use envelope_instance::{bases, populate_envelope};
use lockstep::{document_paths, missing_terms, scope_paths};

/// The inline term map of the passport context.
fn base_terms() -> Map<String, Value> {
    context_value()
        .as_array()
        .expect("@context is an array")
        .iter()
        .find_map(|entry| entry.as_object())
        .expect("the context carries an inline term map")
        .clone()
}

/// The envelope keys whose members are not this table's: `productGroupData` has
/// its own test, and the product identifier is written by hand beneath it.
const ELSEWHERE: &[&str] = &["productGroupData", "productIdentifier"];

/// One populated passport per basis the build models, as its wire form with
/// `productGroupData` left out.
fn populated() -> Vec<Map<String, Value>> {
    let group_data: ProductGroupData = serde_json::from_value(json!({
        "productGroup": "packaging",
        "materialFamily": "placeholder",
    }))
    .expect("an unmodelled product group deserialises");

    bases()
        .into_iter()
        .map(|basis| {
            let mut passport: Passport = base_passport(
                ProductGroup::from_wire_tag("packaging"),
                group_data.clone(),
                "1.0.0",
            );
            populate_envelope(&mut passport, basis);
            let mut wire = serde_json::to_value(&passport)
                .expect("a passport serialises")
                .as_object()
                .expect("a passport is an object")
                .clone();
            wire.remove("productGroupData");
            wire
        })
        .collect()
}

/// 🚨 Every member of every nested envelope object has a term at its position.
#[test]
fn every_member_of_every_envelope_object_has_a_term_at_its_position() {
    let terms = base_terms();

    for (index, passport) in populated().into_iter().enumerate() {
        let mut missing = Vec::new();
        missing_terms(&passport, &terms, "", &mut missing);
        assert!(
            missing.is_empty(),
            "basis #{index}: keys with no JSON-LD term at their position, which a processor drops \
             with everything inside them: {missing:?}"
        );
    }
}

/// Every term the table defines is a member some passport carries.
///
/// The other direction of the check above. A term whose member does not exist is
/// inert and claims to define something: it is what is left behind when a field
/// is renamed and the vocabulary does not follow.
#[test]
fn every_envelope_term_is_a_member_a_passport_carries() {
    let mut defined = BTreeSet::new();
    for (key, definition) in base_terms() {
        if ELSEWHERE.contains(&key.as_str()) {
            continue;
        }
        if let Some(scope) = definition.get("@context").and_then(Value::as_object) {
            scope_paths(scope, &key, &mut defined);
        }
    }

    let mut carried = BTreeSet::new();
    for passport in populated() {
        document_paths(&passport, "", &mut carried);
    }

    let inert: Vec<&String> = defined.difference(&carried).collect();
    assert!(
        inert.is_empty(),
        "terms for members no populated passport carries: {inert:?}"
    );
    assert!(
        defined.len() > 50,
        "only {} terms found: the table is not what this test reads",
        defined.len()
    );
}

/// Each term's IRI is the member's own path from the envelope, and each scope
/// switches propagation off.
///
/// The IRI is the term's identity, so one that disagrees with its own key is not a
/// typo to fix later. And a scope that propagated would make the position check
/// above vacuous, since a term misplaced one level up would still expand beneath
/// it.
#[test]
fn every_scope_is_exact_and_every_iri_is_its_members_path() {
    fn check(scope: &Map<String, Value>, path: &str, scopes: &mut usize) {
        *scopes += 1;
        assert_eq!(
            scope.get("@propagate"),
            Some(&Value::Bool(false)),
            "the scope at `{path}` propagates, so a term misplaced above it would still expand"
        );
        for (key, definition) in scope.iter().filter(|(key, _)| !key.starts_with('@')) {
            let here = format!("{path}.{key}");
            let iri = definition
                .as_str()
                .or_else(|| definition.get("@id").and_then(Value::as_str))
                .unwrap_or_else(|| panic!("`{here}` is defined without an IRI"));
            assert_eq!(
                iri,
                format!("dpp:{here}"),
                "`{here}` maps to an IRI that is not its own path"
            );
            if let Some(inner) = definition.get("@context").and_then(Value::as_object) {
                check(inner, &here, scopes);
            }
        }
    }

    let mut scopes = 0;
    for (key, definition) in base_terms() {
        if ELSEWHERE.contains(&key.as_str()) {
            continue;
        }
        if let Some(scope) = definition.get("@context").and_then(Value::as_object) {
            check(scope, &key, &mut scopes);
        }
    }
    assert!(
        scopes > 12,
        "read {scopes} scopes: the walk is not reaching the nested ones"
    );
}

/// The two members whose keys no schema fixes are carried whole, as
/// `personalData` is.
///
/// A `receipt` holds whatever a calculation recorded, and `disclosureSignatures`
/// is keyed by the dotted path of the field each signature covers. Neither key is
/// a term or an IRI, so as plain terms a processor would drop all of them.
#[test]
fn members_whose_keys_are_data_are_json_literals() {
    let terms = base_terms();

    assert_eq!(
        terms["disclosureSignatures"],
        json!({ "@id": "dpp:disclosureSignatures", "@type": "@json" })
    );
    assert_eq!(
        terms["complianceResult"]["@context"]["receipt"],
        json!({ "@id": "dpp:complianceResult.receipt", "@type": "@json" })
    );
}
