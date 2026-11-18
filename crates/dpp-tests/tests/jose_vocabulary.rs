//! The top-level members `dpp-vc` lets a VC-JOSE-COSE credential or presentation be
//! signed with are the ones the published VC Data Model 2.0 context defines.
//!
//! `dpp_vc::jose::CREDENTIAL_TERMS` and `PRESENTATION_TERMS` are written out in
//! `dpp-vc`, which cannot read the context at build time. This test derives both
//! lists again from the vendored copy of the context and compares them, so a list
//! that drifts from the published document, in either direction, fails here.
//!
//! The derivation: a type's top-level terms are the keys of the scoped `@context`
//! of its definition (`VerifiableCredential` or `VerifiablePresentation`), without
//! the `@protected` flag that is not a term, plus the keyword `@context`. The scoped
//! context carries `id` and `type`, the aliases of `@id` and `@type`, so they come
//! with the keys.

use std::collections::BTreeSet;

use dpp_vc::jose::{CREDENTIAL_TERMS, PRESENTATION_TERMS};
use serde_json::Value;

const CONTEXT: &str = include_str!("../fixtures/jsonld/credentials-v2.jsonld");

/// The terms the context scopes to the definition of `type_name`.
fn scoped_terms(type_name: &str) -> BTreeSet<String> {
    let document: Value = serde_json::from_str(CONTEXT).expect("the vendored context is JSON");
    let scoped = document["@context"][type_name]["@context"]
        .as_object()
        .unwrap_or_else(|| panic!("{type_name} has no scoped @context"));
    let mut terms: BTreeSet<String> = scoped
        .keys()
        .filter(|key| key.as_str() != "@protected")
        .cloned()
        .collect();
    terms.insert("@context".to_owned());
    terms
}

fn listed(terms: &[&str]) -> BTreeSet<String> {
    let set: BTreeSet<String> = terms.iter().map(|t| (*t).to_owned()).collect();
    assert_eq!(set.len(), terms.len(), "a term is listed twice: {terms:?}");
    set
}

#[test]
fn the_credential_terms_are_the_ones_the_context_scopes_to_verifiable_credential() {
    let derived = scoped_terms("VerifiableCredential");

    assert!(derived.contains("credentialSubject") && derived.contains("issuer"));
    assert_eq!(listed(&CREDENTIAL_TERMS), derived);
}

#[test]
fn the_presentation_terms_are_the_ones_the_context_scopes_to_verifiable_presentation() {
    let derived = scoped_terms("VerifiablePresentation");

    assert!(derived.contains("verifiableCredential") && derived.contains("holder"));
    assert_eq!(listed(&PRESENTATION_TERMS), derived);
}
