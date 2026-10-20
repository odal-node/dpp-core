//! The terms for the members of the envelope's own objects.
//!
//! # What this covers
//!
//! The envelope names a term for every key of a passport, and several of those
//! keys hold an object or a list of objects: the manufacturer, the materials, the
//! compliance result and the rest. A JSON-LD term is reachable only at the
//! position its key occupies, so a member of one of these objects has no term
//! unless one is defined *there*, and a key with no term is dropped on expansion
//! together with everything inside it, with no error.
//!
//! `productGroupData` is not here: its members come from the product-group
//! schemas, and [`super::product_terms`] generates them.
//!
//! # Why this is a table
//!
//! These objects are Rust types, not schemas, so there is nothing for a reader to
//! generate from. The table is what `PASSPORT_TERMS` is for the keys of the
//! envelope itself: a list a test compares against the thing it describes.
//! `jsonld_envelope_terms.rs` serialises a passport whose every nested object is
//! built with every field set, written as a struct literal so that adding a field
//! to one of those types does not compile until the test names it, and then holds
//! this table to what comes out, both ways. A field with no term fails it, and so
//! does a term for a field that does not exist.
//!
//! # The shape
//!
//! The same as the product group's: a scoped context on the key, a scope for each
//! object beneath it, and `@propagate: false` on every one, so a term is active at
//! exactly the position its key occupies. `scheme` is a member of `facility` and
//! of the product identifier, and means a different thing in each; `name` is a
//! member of the manufacturer, of each material and of the operator.
//!
//! A term's IRI is `dpp:` and the member's dotted path from the envelope, so
//! `manufacturer.name` is `dpp:manufacturer.name`. Like every term here it names
//! this library's concept and nothing about another vocabulary.
//!
//! # What is a literal
//!
//! A member whose value is a JSON document of the caller's own shape, with keys
//! no schema fixes, is typed `@json` and carried whole, as `personalData` is. That
//! is the compliance result's `receipt`, which holds whatever a calculation
//! recorded, and `disclosureSignatures`, which is keyed by the dotted path of the
//! field each signature covers. Neither's keys are terms or IRIs, so as plain
//! terms they would be dropped.

use serde_json::{Map, Value, json};

/// What a member of an envelope object is.
enum Member {
    /// A scalar, or a list of scalars.
    Leaf,
    /// A JSON value kept whole, as a literal.
    Json,
    /// An object, or a list of objects, with members of its own.
    Object(&'static [(&'static str, Member)]),
}

use Member::{Json, Leaf, Object};

/// The members of a finding in a compliance result and in a lint result.
const FINDING: &[(&str, Member)] = &[("code", Leaf), ("field", Leaf), ("message", Leaf)];

/// A reference to another passport, as a derivation and a component both carry it.
const PASSPORT_REF: &[(&str, Member)] = &[("uri", Leaf), ("publicJwsHash", Leaf)];

/// Every envelope key that holds an object or a list of objects, and its members.
///
/// Keys in alphabetical order, as `PASSPORT_TERMS` keeps them. A key that holds
/// only scalars is not here, and `productGroupData` and `personalData` have terms
/// of their own elsewhere.
const NESTED: &[(&str, &[(&str, Member)])] = &[
    (
        "applicableInstruments",
        &[("instrument", Leaf), ("recorded", Leaf)],
    ),
    (
        "co2ePerUnit",
        &[
            ("valueKg", Leaf),
            ("lifecycleStage", Leaf),
            ("systemBoundary", Leaf),
            ("methodologyRef", Leaf),
            ("performanceClass", Leaf),
        ],
    ),
    (
        "complianceResult",
        &[
            ("co2eScore", Leaf),
            ("repairabilityIndex", Leaf),
            ("recycledContentPct", Leaf),
            ("complianceStatus", Leaf),
            ("violations", Object(FINDING)),
            ("warnings", Object(FINDING)),
            ("rulesetVersion", Leaf),
            ("assessedAt", Leaf),
            ("receipt", Json),
        ],
    ),
    (
        "componentRefs",
        &[
            ("reference", Object(PASSPORT_REF)),
            ("quantity", Object(&[("value", Leaf), ("unit", Leaf)])),
            ("role", Leaf),
        ],
    ),
    (
        "derivedFrom",
        &[("reference", Object(PASSPORT_REF)), ("operation", Leaf)],
    ),
    (
        "facility",
        &[
            ("scheme", Leaf),
            ("value", Leaf),
            ("name", Leaf),
            ("country", Leaf),
            ("address", Leaf),
        ],
    ),
    (
        "lintResult",
        &[
            ("packVersion", Leaf),
            (
                "findings",
                Object(&[
                    ("code", Leaf),
                    ("field", Leaf),
                    ("severity", Leaf),
                    ("message", Leaf),
                ]),
            ),
            ("assessedAt", Leaf),
        ],
    ),
    (
        "manufacturer",
        &[
            ("name", Leaf),
            ("address", Leaf),
            ("registeredTradeName", Leaf),
            ("electronicAddress", Leaf),
            ("country", Leaf),
            ("didWebUrl", Leaf),
        ],
    ),
    (
        "materials",
        &[
            ("name", Leaf),
            ("weightKg", Leaf),
            ("recycledPct", Leaf),
            ("countryOfOrigin", Leaf),
        ],
    ),
    (
        "repairabilityScore",
        &[
            ("overall", Leaf),
            (
                "criteria",
                Object(&[("name", Leaf), ("score", Leaf), ("weight", Leaf)]),
            ),
        ],
    ),
    (
        "responsibleOperator",
        &[
            (
                "operator",
                Object(&[
                    ("did", Leaf),
                    ("name", Leaf),
                    ("role", Leaf),
                    ("euOperatorId", Leaf),
                    ("euOperatorIdScheme", Leaf),
                    ("country", Leaf),
                    ("registeredTradeName", Leaf),
                    ("postalAddress", Leaf),
                    ("electronicAddress", Leaf),
                ]),
            ),
            // A basis is a string, except the one that names the law, which is
            // an object with the citation inside it.
            (
                "basis",
                Object(&[("otherUnionLaw", Object(&[("citation", Leaf)]))]),
            ),
        ],
    ),
    (
        "seal",
        &[
            ("format", Leaf),
            ("sealValue", Leaf),
            ("signingCertRef", Leaf),
            ("conformanceLevel", Leaf),
            ("sealedAt", Leaf),
            ("placeholder", Leaf),
        ],
    ),
];

/// The scoped context for the members of the envelope key `key`, or `None` when
/// the key holds no object of its own.
pub(super) fn scope_of(key: &str) -> Option<Value> {
    NESTED
        .iter()
        .find(|(name, _)| *name == key)
        .map(|(name, members)| Value::Object(scope(members, name)))
}

/// The scoped context for `members`, found at `path` from the envelope.
fn scope(members: &[(&str, Member)], path: &str) -> Map<String, Value> {
    // 🚨 Without this a scope would propagate to everything beneath it, and a
    // term misplaced one level up would still expand: the position would stop
    // being checked by anything.
    let mut terms = Map::new();
    terms.insert("@propagate".to_owned(), json!(false));

    for (name, member) in members {
        let member_path = format!("{path}.{name}");
        let iri = format!("dpp:{member_path}");
        let definition = match member {
            Leaf => json!(iri),
            Json => json!({ "@id": iri, "@type": "@json" }),
            Object(inner) => {
                json!({ "@id": iri, "@context": Value::Object(scope(inner, &member_path)) })
            }
        };
        terms.insert((*name).to_owned(), definition);
    }
    terms
}
