//! The JSON-LD context references only remote contexts that resolve.
//!
//! A string entry in an `@context` array is fetched by the consumer at
//! expansion time. One that 404s fails a conforming processor outright and
//! makes a lenient one drop every term it cannot define — and since passport
//! payloads use bare keys, that leaves the `ld+json` door conveying no linked
//! data at all. Two such entries existed simultaneously before this: one in
//! this crate, one hand-rolled in the resolver, each pointing at a different
//! dead URL.

use dpp_domain::{Gtin, PassportCredential, ProductIdentifier};
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

    for term in ["dpp", "productIdentifier", "productGroup", "passportId"] {
        assert!(
            inline.get(term).is_some(),
            "term '{term}' is not defined inline — a consumer would have to \
             fetch it from somewhere"
        );
    }

    // 🚨 `gtin` is inline too, but **scoped to `productIdentifier`**, which is
    // where the key now is. This loop used to look for it at the top level,
    // and that is precisely the assertion that kept passing while the
    // credential stopped carrying the identifier at all: a term is reachable
    // only at the position its key occupies.
    assert!(
        inline["productIdentifier"]["@context"]
            .get("gtin")
            .is_some(),
        "the GS1 term must be defined where the key is"
    );
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

/// 🚨 Every key a product identifier carries has a term, at the position it
/// occupies.
///
/// A JSON-LD term is only reachable where the key actually is. When the
/// identifier moved from a bare `gtin` at the top of `productGroupData` to a
/// `productIdentifier` object, the `gs1:gtin` term stayed behind and
/// `productIdentifier` had no definition — so expansion dropped the node and
/// everything inside it. Measured against a processor before the fix: the old
/// shape expanded `productGroupData` to `{"https://ref.gs1.org/voc/gtin": …}`,
/// the current one to `{}`. A credential whose semantic form carried no
/// product identity at all, and nothing said so.
///
/// This asserts the terms rather than running an expansion, because the
/// workspace has no JSON-LD processor — so it catches the drift that caused
/// the defect without claiming to verify expansion. Two things are checked,
/// because either alone lets the defect back: that every key the wire form
/// actually carries has a term, and that each term names the IRI it should.
/// A term aimed at the wrong IRI is as wrong as no term and is just as quiet.
///
/// 🚨 The key set is read off the serialised variants, never listed here. A
/// literal list is the same mistake one level up — an assertion pinned to the
/// keys that existed when it was written, which is precisely how the old
/// top-level `gtin` check went on passing after the key moved.
#[test]
fn every_product_identifier_key_has_a_scoped_term() {
    let ctx = context_value();
    let terms = ctx
        .as_array()
        .expect("@context is an array")
        .iter()
        .find(|e| e.is_object())
        .and_then(Value::as_object)
        .expect("the context carries an inline term map");

    let scoped = terms["productIdentifier"]["@context"]
        .as_object()
        .expect("productIdentifier defines a scoped context, or its node is dropped");

    // 🚨 The keys are read off the serialised variants rather than listed
    // here. A literal list would be one more assertion pinned to a position:
    // a fourth clause 5 scheme would add a key, the list would not know, and
    // the new key would be dropped on expansion exactly as `gtin` was.
    for identifier in [
        ProductIdentifier::gs1(Gtin::parse("09506000134352").expect("valid GTIN literal")),
        ProductIdentifier::identification_link("https://example.com/p/1")
            .expect("valid identification link"),
        ProductIdentifier::did("did:web:example.com:p:1").expect("valid DID"),
    ] {
        let wire = serde_json::to_value(&identifier).expect("a product identifier serialises");
        for key in wire.as_object().expect("a tagged object").keys() {
            assert!(
                scoped.contains_key(key),
                "`{key}` appears inside productIdentifier and has no term: {scoped:?}"
            );
        }
    }

    // Presence alone is not enough: a term aimed at the wrong IRI passes every
    // check above and still says something false about the identifier.
    for (key, iri) in [
        ("scheme", "dpp:identifierScheme"),
        ("gtin", "gs1:gtin"),
        ("url", "dpp:identificationLink"),
        ("did", "dpp:decentralizedIdentifier"),
    ] {
        assert_eq!(scoped[key], json!(iri), "`{key}` maps to the wrong IRI");
    }

    // 🚨 And they must stay scoped. `scheme` is also a facility-snapshot field,
    // so a global term would file a GLN scheme under the product identifier's
    // meaning — two different things collapsed into one IRI.
    for key in ["scheme", "url", "did"] {
        assert!(
            !terms.contains_key(key),
            "`{key}` must not be a global term; it means different things elsewhere"
        );
    }

    // Scoped contexts are a JSON-LD 1.1 feature; without the version marker a
    // processor treats the inner `@context` as an error rather than a scope.
    assert_eq!(terms["@version"], json!(1.1));
}
