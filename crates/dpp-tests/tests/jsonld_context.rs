//! The JSON-LD context references only remote contexts that resolve.
//!
//! A string entry in an `@context` array is fetched by the consumer at
//! expansion time. One that 404s fails a conforming processor outright and
//! makes a lenient one drop every term it cannot define — and since passport
//! payloads use bare keys, that leaves the `ld+json` door conveying no linked
//! data at all. Two such entries existed simultaneously before this: one in
//! this crate, one hand-rolled in the resolver, each pointing at a different
//! dead URL.

use dpp_vc::{REMOTE_CONTEXTS, context_value, frame_passport, passport_context, strip_context};
use serde_json::{Value, json};

/// Remote contexts confirmed to resolve, with the date checked.
///
/// Deliberately a hardcoded list rather than a live fetch: the main gate must
/// not depend on the network. The value is that adding a remote context means
/// editing this list, which is the moment someone checks the URL.
const VERIFIED_RESOLVABLE: &[(&str, &str)] = &[("https://www.w3.org/ns/did/v1", "2026-07-30")];

/// Every string entry in the context is one somebody has confirmed resolves.
#[test]
fn every_remote_context_is_verified_resolvable() {
    let ctx = context_value();
    let entries = ctx.as_array().expect("@context is an array");

    for entry in entries {
        let Value::String(url) = entry else {
            continue; // inline term maps carry no fetch obligation
        };
        assert!(
            VERIFIED_RESOLVABLE.iter().any(|(known, _)| known == url),
            "'{url}' is referenced as a remote context but is not in the \
             verified-resolvable list. Confirm it resolves, add it with the date \
             checked, or inline the terms instead."
        );
    }
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

    for term in ["dpp", "gs1", "schema", "gtin", "sector", "passportId"] {
        assert!(
            inline.get(term).is_some(),
            "term '{term}' is not defined inline — a consumer would have to \
             fetch it from somewhere"
        );
    }
}

/// No dead URL from either previous definition comes back.
#[test]
fn the_withdrawn_context_urls_stay_out() {
    let serialised = serde_json::to_string(&passport_context()).expect("serialises");
    for dead in [
        "https://odal-node.io/schemas/dpp/v1",
        "https://ref.gs1.org/standards/digital-link/context/",
    ] {
        assert!(
            !serialised.contains(dead),
            "'{dead}' returned 404 when checked and must not be referenced as a \
             remote context"
        );
    }
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
