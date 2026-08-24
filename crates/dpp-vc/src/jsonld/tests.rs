//! JSON-LD frame/strip round-trip tests.

use serde_json::json;

use super::*;

#[test]
fn frame_and_strip_round_trip() {
    let passport = json!({ "passportId": "abc", "productGroup": "battery" });
    let framed = frame_passport(passport.clone());
    let stripped = strip_context(framed);
    assert_eq!(stripped["passportId"], "abc");
    assert!(stripped.get("@context").is_none());
}

#[test]
fn frame_passport_preserves_non_object_payload() {
    // A non-object payload can't be merged into the context map, but it must be
    // returned intact rather than silently discarded into a bare envelope.
    let framed = frame_passport(json!("not-an-object"));
    assert_eq!(framed, json!("not-an-object"));
}

#[test]
fn strip_context_passes_through_non_object() {
    let array = json!(["a", "b"]);
    assert_eq!(strip_context(array.clone()), array);
}

// ─── Prefix provenance ───────────────────────────────────────────────────────

/// The term map emitted in `@context`, as `(term, target)` pairs.
fn term_map() -> serde_json::Map<String, serde_json::Value> {
    let context = context_value();
    let entries = context.as_array().expect("@context is an array");
    entries
        .iter()
        .find_map(|e| e.as_object())
        .expect("@context carries an inline term object")
        .clone()
}

/// Every prefix this context declares is either ours or backed by a record that
/// permits emission.
///
/// # What this closes
///
/// The prefix IRIs used to be literals here, with their provenance in a doc
/// comment. `dpp-vocab` exists because that class of claim was being kept at two
/// different standards of rigour — `dpp-aas` provenanced and gated, this one
/// neither — and the crate was carved to unify them. `dpp-aas` migrated; this
/// side did not, so the register had a consumer and a hold-out.
///
/// A doc comment is not a record. It cannot be queried, it does not carry a
/// `checkedOn` date the build can read, and nothing fails when the thing it
/// describes changes underneath it.
#[test]
fn every_declared_prefix_is_ours_or_permitted_by_a_record() {
    use dpp_vocab::{OWN_JSONLD_NAMESPACE, VocabularyRegister};

    let register = VocabularyRegister::new();
    let map = term_map();

    // A prefix declaration is a term whose target is an absolute IRI; a term
    // mapping (`gtin: gs1:gtin`) is compact and checked separately below.
    let prefixes: Vec<(String, String)> = map
        .iter()
        .filter_map(|(term, target)| {
            let target = target.as_str()?;
            (target.starts_with("http://") || target.starts_with("https://"))
                .then(|| (term.clone(), target.to_owned()))
        })
        .collect();

    assert!(
        !prefixes.is_empty(),
        "the context declares no prefixes at all"
    );

    for (term, iri) in prefixes {
        if iri == OWN_JSONLD_NAMESPACE {
            continue; // ours; needs no record
        }
        let record = register
            .all()
            .iter()
            .find(|v| v.contains(&iri))
            .unwrap_or_else(|| {
                panic!(
                    "prefix '{term}' declares {iri}, which no vocabulary record covers — \
                     add a record before naming someone else's vocabulary"
                )
            })
            .clone();
        assert!(
            record.permits_emission(),
            "prefix '{term}' declares {iri} from the '{}' record, which does not permit \
             emission ({:?}/{:?})",
            record.key,
            record.status,
            record.layer
        );
    }
}

/// Every compact term expands to an IRI the register permits.
///
/// The prefix check above is not sufficient on its own: declaring a permitted
/// prefix and then coining a term under it is still a claim about that
/// authority's vocabulary. This expands each compact value the way a JSON-LD
/// processor would and puts the result to the register.
#[test]
fn every_compact_term_expands_to_a_permitted_iri() {
    use dpp_vocab::{OWN_JSONLD_NAMESPACE, VocabularyRegister};

    let register = VocabularyRegister::new();
    let map = term_map();

    let prefixes: std::collections::HashMap<String, String> = map
        .iter()
        .filter_map(|(term, target)| {
            let target = target.as_str()?;
            (target.starts_with("http")).then(|| (term.clone(), target.to_owned()))
        })
        .collect();

    let mut checked = 0usize;
    for (term, target) in &map {
        let Some(target) = target.as_str() else {
            continue;
        };
        let Some((prefix, suffix)) = target.split_once(':') else {
            continue;
        };
        let Some(base) = prefixes.get(prefix) else {
            continue; // not a compact form against a declared prefix
        };
        let expanded = format!("{base}{suffix}");
        checked += 1;

        if expanded.starts_with(OWN_JSONLD_NAMESPACE) {
            continue;
        }
        assert!(
            register.verdict(&expanded).is_permitted(),
            "term '{term}' expands to {expanded}, which the register does not permit: {}",
            register.verdict(&expanded).reason()
        );
    }

    assert!(
        checked > 0,
        "no compact terms were expanded — the check would pass vacuously"
    );
}

/// The `gs1:` prefix in the context is the one the record carries.
///
/// Pins the derivation rather than the value: if the record's `namespaceIri`
/// moves, the context must move with it, and a literal here would silently not.
#[test]
fn the_gs1_prefix_matches_its_record() {
    use dpp_vocab::VocabularyRegister;

    let register = VocabularyRegister::new();
    let recorded = register
        .all()
        .iter()
        .find(|v| v.key == "gs1")
        .expect("gs1 record")
        .namespace_iri
        .clone()
        .expect("gs1 record carries a namespaceIri");

    assert_eq!(
        term_map()["gs1"].as_str().expect("gs1 prefix is a string"),
        recorded,
        "the context must declare the prefix the record records, not a copy of it"
    );
}
