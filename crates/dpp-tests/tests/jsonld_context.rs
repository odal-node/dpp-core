//! The JSON-LD context references only remote contexts that resolve.
//!
//! A string entry in an `@context` array is fetched by the consumer at
//! expansion time. One that 404s fails a conforming processor outright and
//! makes a lenient one drop every term it cannot define — and since passport
//! payloads use bare keys, that leaves the `ld+json` door conveying no linked
//! data at all. Two such entries existed simultaneously before this: one in
//! this crate, one hand-rolled in the resolver, each pointing at a different
//! dead URL.

use dpp_domain::PassportCredential;
use dpp_vc::credential::{CredentialBuilder, CredentialRole, DppCredentialSubject};
use dpp_vc::{REMOTE_CONTEXTS, context_value, frame_passport, passport_context, strip_context};
use serde_json::{Value, json};

/// Remote contexts confirmed to resolve, with the date checked.
///
/// Deliberately a hardcoded list rather than a live fetch: the main gate must
/// not depend on the network. The value is that adding a remote context means
/// editing this list, which is the moment someone checks the URL.
///
/// Workspace-wide, not just `dpp_vc::jsonld`: every `@context` array the
/// workspace can emit is checked against this same list, in
/// `every_context_array_the_workspace_can_emit_is_verified_resolvable` below.
const VERIFIED_RESOLVABLE: &[(&str, &str)] = &[
    ("https://www.w3.org/ns/did/v1", "2026-07-30"),
    ("https://www.w3.org/ns/credentials/v2", "2026-08-07"),
];

/// Fail on any string `@context` entry not in [`VERIFIED_RESOLVABLE`]. Object
/// entries (inline term maps) carry no fetch obligation and are skipped.
fn assert_verified_resolvable(label: &str, entries: &[Value]) {
    for entry in entries {
        let Value::String(url) = entry else {
            continue;
        };
        assert!(
            VERIFIED_RESOLVABLE.iter().any(|(known, _)| known == url),
            "{label}: '{url}' is referenced as a remote context but is not in \
             VERIFIED_RESOLVABLE. Confirm it resolves, add it with the date \
             checked, or inline the terms instead."
        );
    }
}

/// Every string entry in the context is one somebody has confirmed resolves.
#[test]
fn every_remote_context_is_verified_resolvable() {
    let ctx = context_value();
    let entries = ctx.as_array().expect("@context is an array");
    assert_verified_resolvable("dpp_vc::jsonld::passport_context", entries);
}

/// Every `@context` array anything in the workspace can emit — not just the
/// passport envelope — references only verified-resolvable remote contexts.
///
/// Both `PassportCredential` and `DppAccessCredential` hard-code their own
/// `@context` array, so neither was covered by the passport-envelope-only
/// check above, which is why each independently grew a dead
/// `schema.odal-node.io` entry. This is the guard that stops a third type
/// from doing the same.
#[test]
fn every_context_array_the_workspace_can_emit_is_verified_resolvable() {
    let passport_credential = PassportCredential::new(
        "did:web:issuer.example.com".into(),
        dpp_domain::PassportCredentialSubject {
            id: "urn:uuid:00000000-0000-0000-0000-000000000000".into(),
            payload_hash: "deadbeef".into(),
        },
    );
    assert_verified_resolvable("PassportCredential", &passport_credential.context);

    let access_credential = CredentialBuilder::new(
        "did:web:authority.example.com".into(),
        DppCredentialSubject {
            id: "did:web:holder.example.com".into(),
            name: "Test Holder".into(),
            role: CredentialRole::AuthorisedRepairer,
            country: "DE".into(),
            product_groups: vec!["textile".into()],
            product_categories: vec![],
        },
    )
    .build();
    assert_verified_resolvable("DppAccessCredential", &access_credential.context);
}

/// The declared list and what is actually emitted cannot drift apart.
#[test]
fn the_declared_remote_context_list_matches_what_is_emitted() {
    let ctx = context_value();
    let emitted: Vec<&str> = ctx
        .as_array()
        .expect("array")
        .iter()
        .filter_map(Value::as_str)
        .collect();

    assert_eq!(
        emitted, REMOTE_CONTEXTS,
        "REMOTE_CONTEXTS disagrees with the context actually built"
    );
}

/// The vocabulary is inlined, so the document defines its own terms.
///
/// Hosting a context document is a commitment to keep a URL alive for as long
/// as any passport references it. Inlining removes that obligation entirely and
/// cannot 404.
#[test]
fn the_passport_vocabulary_is_inlined() {
    let ctx = context_value();
    let inline = ctx
        .as_array()
        .expect("array")
        .iter()
        .find(|e| e.is_object())
        .expect("the context carries an inline term map");

    for term in ["dpp", "gtin", "productGroup", "passportId"] {
        assert!(
            inline.get(term).is_some(),
            "term '{term}' is not defined inline — a consumer would have to \
             fetch it from somewhere"
        );
    }
}

/// No dead URL from any previous definition comes back, in any context array.
#[test]
fn the_withdrawn_context_urls_stay_out() {
    let passport_credential = PassportCredential::new(
        "did:web:issuer.example.com".into(),
        dpp_domain::PassportCredentialSubject {
            id: "urn:uuid:00000000-0000-0000-0000-000000000000".into(),
            payload_hash: "deadbeef".into(),
        },
    );
    let access_credential = CredentialBuilder::new(
        "did:web:authority.example.com".into(),
        DppCredentialSubject {
            id: "did:web:holder.example.com".into(),
            name: "Test Holder".into(),
            role: CredentialRole::AuthorisedRepairer,
            country: "DE".into(),
            product_groups: vec!["textile".into()],
            product_categories: vec![],
        },
    )
    .build();

    let serialised = [
        serde_json::to_string(&passport_context()).expect("serialises"),
        serde_json::to_string(&passport_credential.context).expect("serialises"),
        serde_json::to_string(&access_credential.context).expect("serialises"),
    ]
    .join("\n");

    for dead in [
        "https://odal-node.io/schemas/dpp/v1",
        "https://ref.gs1.org/standards/digital-link/context/",
        "https://schema.odal-node.io/credentials/dpp-passport/v1",
        "https://schema.odal-node.io/credentials/dpp-access/v1",
    ] {
        assert!(
            !serialised.contains(dead),
            "'{dead}' does not resolve — no DNS record — and must not be \
             referenced as a remote context"
        );
    }
}

/// Every prefix this context declares is either our own namespace or a
/// vocabulary `dpp-vocab` records as verified — the same rule `dpp-aas`
/// enforces for `semanticId`, extended to the JSON-LD door.
///
/// `gs1:` and `schema:` were both declared here with no provenance record, and
/// both were removed in favour of `dpp:`-only terms. `gs1:` has since come back
/// — GS1's `gtin` definition was read on 2026-08-11 and the vocabulary is
/// `verified` in `dpp-vocab` — and it came back *through* this gate rather than
/// around it, which is the case the gate was written for. `schema:` is still
/// refused: Schema.org is `tracked`, evaluated and not adopted.
///
/// A term that maps to another declared prefix (e.g. `"gtin": "gs1:gtin"`) is
/// not itself a namespace declaration and is skipped — only entries whose value
/// is an absolute IRI declare one.
#[test]
fn every_declared_prefix_is_ours_or_verified() {
    // This crate's own JSON-LD vocabulary — a dereferenceable-shaped IRI by
    // JSON-LD convention, distinct from `dpp_vocab::OWN_NAMESPACE`
    // (`urn:odal-node:`), which is `dpp-aas`'s `semanticId` scheme. Two
    // conventions for one project, each right for its own door.
    const OWN_JSONLD_NAMESPACE: &str = "https://schema.odal-node.io/dpp#";

    let ctx = context_value();
    let inline = ctx
        .as_array()
        .expect("array")
        .iter()
        .find(|e| e.is_object())
        .expect("the context carries an inline term map")
        .as_object()
        .expect("the inline entry is an object");

    let register = dpp_vocab::VocabularyRegister::new();
    let mut checked = 0usize;
    for (term, value) in inline {
        let Some(iri) = value.as_str() else { continue };
        if !iri.contains("://") {
            continue; // a compact-IRI term alias, not a prefix declaration
        }
        checked += 1;
        let verdict = register.verdict(iri);
        assert!(
            iri.starts_with(OWN_JSONLD_NAMESPACE) || verdict.is_permitted(),
            "term '{term}' declares prefix '{iri}', which is neither our own \
             namespace nor a verified vocabulary: {}",
            verdict.reason()
        );
    }
    assert!(
        checked > 0,
        "the gate asserted nothing — no prefix declarations found"
    );
}

/// Framing keeps the payload intact and stripping is its inverse.
#[test]
fn framing_round_trips_a_passport() {
    let passport = json!({ "id": "urn:dpp:abc", "productName": "EcoCell" });
    let framed = frame_passport(passport.clone());

    assert!(framed.get("@context").is_some(), "framing adds the context");
    assert_eq!(framed["productName"], "EcoCell");
    assert_eq!(strip_context(framed), passport);
}
