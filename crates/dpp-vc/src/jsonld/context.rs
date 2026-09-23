//! JSON-LD context envelope: build / frame / strip a DPP passport payload.

use std::sync::OnceLock;

use dpp_vocab::{OWN_JSONLD_NAMESPACE, VocabularyRegister};
use serde_json::{Value, json};

/// The `gs1:` prefix IRI, read from the register rather than written here.
///
/// # Why this is not a literal
///
/// It was one, and the doc comment below carried its provenance — the date GS1's
/// definition was read, what it said, the licence. That is the right *content*
/// filed in the wrong *place*: a doc comment is not a record, nothing checks it,
/// and `dpp-vocab` exists precisely so this class of claim has one home with a
/// source and a `checkedOn` date. The register was created to unify two
/// mechanisms held to different standards of rigour; this was the one still on
/// the looser standard.
///
/// # Panics
///
/// If the `gs1` record is absent, carries no `namespaceIri`, or no longer
/// permits emission. All three are build-time facts about files embedded in
/// `dpp-vocab`, and every one of them means this context must stop claiming
/// GS1's vocabulary. Emitting a prefix the register refuses is the failure this
/// arrangement exists to prevent, so it is not papered over with a fallback.
fn gs1_namespace() -> &'static str {
    static GS1: OnceLock<String> = OnceLock::new();
    GS1.get_or_init(|| {
        let register = VocabularyRegister::new();
        let record = register
            .all()
            .iter()
            .find(|v| v.key == "gs1")
            .expect("the gs1 record is embedded in dpp-vocab")
            .clone();
        assert!(
            record.permits_emission(),
            "the gs1 record no longer permits emission ({:?}/{:?}), so this context              must not declare a gs1: prefix",
            record.status,
            record.layer
        );
        record
            .namespace_iri
            .expect("the gs1 record carries a namespaceIri")
    })
    .as_str()
}

/// Remote contexts this passport context references.
///
/// **A string entry in an `@context` array is fetched by the consumer at
/// expansion time.** One that does not resolve is not cosmetic: a conforming
/// processor fails the whole document with a remote-context load error, and a
/// lenient one drops every term it cannot define. Since our payload uses bare
/// keys, that means the `ld+json` door would convey no linked data at all —
/// worse than serving plain JSON, because the `@context` is itself a claim that
/// the document is semantically resolvable.
///
/// So this list is deliberately short and deliberately explicit: adding to it
/// means editing this constant *and* the test that pins it, which is the point
/// at which someone checks the URL. Two entries were removed on 2026-07-30 for
/// returning 404 — `https://ref.gs1.org/standards/digital-link/context/`, which
/// this crate referenced, and `https://odal-node.io/schemas/dpp/v1`, which the
/// resolver hand-rolled.
///
/// Term-to-IRI mappings are a different matter and are inlined below: a prefix
/// IRI names a vocabulary and is never dereferenced during expansion, so it
/// carries no such obligation.
pub const REMOTE_CONTEXTS: &[&str] = &["https://www.w3.org/ns/did/v1"];

/// Build the JSON-LD context for an Odal Node passport.
///
/// The vocabulary is **inlined** rather than hosted. Hosting a context document
/// is a commitment to keep a URL resolving for as long as any passport
/// referencing it exists — years, under ESPR retention — and that is an
/// operational obligation, not a library decision. An inline term map cannot
/// 404, and it can be adopted later without invalidating passports issued now.
///
/// Every term maps to our own `dpp:` prefix, with one exception.
///
/// `gtin`, `createdAt` and `updatedAt` all used to borrow GS1's and
/// Schema.org's prefixes (`gs1:gtin`, `schema:dateCreated`,
/// `schema:dateModified`) with no provenance record, which is exactly the
/// unsupported claim `dpp-vocab`'s rule exists to catch. All three were
/// withdrawn to `dpp:` on 2026-08-10.
///
/// **`gtin` is back, and only `gtin`.** Its record is `vocabularies/gs1.json`
/// in `dpp-vocab`, carrying what was read, when, and under what licence —
/// `gs1_namespace` reads the prefix IRI from there rather than repeating it,
/// so the claim and its evidence cannot drift apart. GS1's `gtin` is
/// [`Gtin`](dpp_domain::Gtin)'s shape exactly, which is why the term says
/// something true.
///
/// 🚨 It is now defined **inside `productIdentifier`**, not at the top level,
/// because that is where the key is. A term is only reachable at the position
/// the key occupies: when the identifier moved under `productIdentifier` and
/// that node had no definition of its own, expansion dropped the node and
/// carried `gtin` down with it. The scheme 2 and 3 terms are scoped the same
/// way, and deliberately are **not** global — `scheme` is also a facility
/// snapshot field, and one global term would give a GLN scheme the product
/// identifier's meaning.
///
/// The other two stay `dpp:`. Schema.org is `tracked` in `dpp-vocab`: evaluated,
/// not adopted, and its record does not permit emission. And note that what was
/// read was **one term**, not the GS1 vocabulary — declaring the `gs1:` prefix
/// is what makes the compact form expand, not a claim that anything else under
/// it has been checked.
///
/// Every key a `Passport` emits, mapped to the IRI that gives it meaning.
///
/// # Why this is a table and not a literal
///
/// A JSON-LD term is reachable only at the position its key occupies, and a key
/// with no term is dropped on expansion together with everything inside it.
/// That makes this list's *completeness* the property that matters, and a list
/// whose completeness matters should be something a test can read. It was a
/// `json!` literal covering six of the envelope's keys, and nothing anywhere
/// compared it against the struct it claims to describe.
/// `every_passport_wire_key_has_a_term` is now that comparison.
///
/// 🚨 Two of the terms it did carry pointed at keys that do not exist.
/// `passportId` was never emitted — the key is `id` — and `jws` was never
/// emitted either, because the fields serialise as `jwsSignature` and
/// `publicJwsSignature`. Both are gone; the signature keys are below under the
/// names they actually have.
///
/// 🚨 `productGroup` expanded to `dpp:product_group` — the only IRI here
/// that disagreed with its own key, alone in snake_case among camelCase
/// neighbours. It is corrected rather than preserved. An IRI *is* a term's
/// identity, so this does change what the term means, and that is affordable
/// exactly once: there are no passports in the field to mean anything
/// different to. Keeping it would have bought compatibility with nobody at the
/// cost of a permanent inconsistency in a published vocabulary.
///
/// Every IRI is `dpp:`. The one foreign prefix in this context is `gs1:gtin`,
/// scoped under `productIdentifier`, and it is foreign only because `dpp-vocab`
/// holds a provenance record for it. `id` is absent here because it aliases the
/// `@id` keyword rather than naming a vocabulary term.
///
/// # These IRIs are names, not addresses
///
/// `dpp:` expands into [`OWN_JSONLD_NAMESPACE`], which does not resolve and is
/// under no obligation to — the reasoning is on the constant itself in
/// `dpp-vocab`, and it is the standing position rather than an oversight. A
/// prefix IRI is concatenated with the term and never fetched, so minting
/// names under it costs nothing and promises nothing. The
/// obligation that *is* real belongs to a **string entry** in the `@context`
/// array, which a consumer does fetch; that is [`REMOTE_CONTEXTS`], and it is
/// deliberately short for exactly this reason. Do not read the one rule onto
/// the other.
const PASSPORT_TERMS: &[(&str, &str)] = &[
    ("applicableInstruments", "dpp:applicableInstruments"),
    ("batchId", "dpp:batchId"),
    ("carrierSerial", "dpp:carrierSerial"),
    ("co2ePerUnit", "dpp:co2ePerUnit"),
    ("commodityCode", "dpp:commodityCode"),
    ("complianceResult", "dpp:complianceResult"),
    ("componentRefs", "dpp:componentRefs"),
    ("createdAt", "dpp:createdAt"),
    ("derivedFrom", "dpp:derivedFrom"),
    ("disclosureSignatures", "dpp:disclosureSignatures"),
    ("facility", "dpp:facility"),
    ("granularity", "dpp:granularity"),
    ("jwsSignature", "dpp:jwsSignature"),
    ("lifeStatus", "dpp:lifeStatus"),
    ("lintResult", "dpp:lintResult"),
    ("manufacturer", "dpp:manufacturer"),
    ("materials", "dpp:materials"),
    ("operatorIdentifier", "dpp:operatorIdentifier"),
    ("placedOnMarketDate", "dpp:placedOnMarketDate"),
    ("productGroup", "dpp:productGroup"),
    ("productGroupData", "dpp:productGroupData"),
    ("productId", "dpp:productId"),
    ("productName", "dpp:productName"),
    ("publicJwsSignature", "dpp:publicJwsSignature"),
    ("publishedAt", "dpp:publishedAt"),
    ("qrCodeUrl", "dpp:qrCodeUrl"),
    ("repairabilityScore", "dpp:repairabilityScore"),
    ("responsibleOperator", "dpp:responsibleOperator"),
    ("retentionLocked", "dpp:retentionLocked"),
    ("retentionUntil", "dpp:retentionUntil"),
    ("schemaVersion", "dpp:schemaVersion"),
    ("seal", "dpp:seal"),
    ("serialNumber", "dpp:serialNumber"),
    ("status", "dpp:status"),
    ("supersedesId", "dpp:supersedesId"),
    ("updatedAt", "dpp:updatedAt"),
    ("version", "dpp:version"),
];

/// The literal is built once and cloned per call — callers extend the
/// returned value (e.g. [`frame_passport`] merges passport fields into it),
/// so it must stay an owned, independently-mutable `Value` per call site.
pub fn passport_context() -> Value {
    static CONTEXT: OnceLock<Value> = OnceLock::new();
    CONTEXT
        .get_or_init(|| {
            let mut terms = serde_json::Map::new();

            // 🚨 JSON-LD 1.1, for the scoped context below. Without it a
            // processor treats `@context` inside a term definition as an error
            // rather than a scope.
            terms.insert("@version".to_owned(), json!(1.1));
            terms.insert("dpp".to_owned(), json!(OWN_JSONLD_NAMESPACE));
            terms.insert("gs1".to_owned(), json!(gs1_namespace()));

            // 🚨 `id` is the passport's own identity, aliased to the `@id`
            // keyword rather than to a `dpp:` IRI. It resolved before only
            // because the remote DID context happens to alias it — an identity
            // that depended on someone else's document keeping its shape. That
            // definition is `"id": "@id"` exactly, so restating it here is the
            // identical redefinition JSON-LD 1.1 permits for a protected term,
            // and the passport now names itself either way.
            terms.insert("id".to_owned(), json!("@id"));

            // 🚨 The identifier's terms are **scoped to it**, not global.
            // `scheme` is also a field on the facility snapshot, so a global
            // term would file a GLN scheme under the product identifier's
            // meaning — two different things collapsed into one IRI.
            //
            // This is where `gtin` lives, because this is where the key is. It
            // used to sit at the top of `productGroupData`, and when the
            // identifier moved under `productIdentifier` the term stayed
            // behind: `productIdentifier` had no definition, so expansion
            // dropped the whole node and everything in it. Measured against a
            // JSON-LD processor — the old shape expanded `productGroupData` to
            // `{"https://ref.gs1.org/voc/gtin": [...]}` and the current one
            // expanded it to `{}`.
            terms.insert(
                "productIdentifier".to_owned(),
                json!({
                    "@id": "dpp:productIdentifier",
                    "@context": {
                        "scheme": "dpp:identifierScheme",
                        "gtin": "gs1:gtin",
                        "url": "dpp:identificationLink",
                        "did": "dpp:decentralizedIdentifier"
                    }
                }),
            );

            for (term, iri) in PASSPORT_TERMS {
                terms.insert((*term).to_owned(), json!(iri));
            }

            json!({
                "@context": [REMOTE_CONTEXTS[0], Value::Object(terms)]
            })
        })
        .clone()
}

/// The `@context` value alone, for a caller that already has a passport object
/// and needs to stamp the context onto it.
///
/// Exists so the resolver stops constructing its own: two definitions of one
/// context is how the served one came to reference a URL that 404s while this
/// one referenced a different URL that also 404s.
pub fn context_value() -> Value {
    passport_context()["@context"].clone()
}

/// Wrap a passport JSON value in a JSON-LD envelope.
///
/// A non-object payload cannot be merged into the `@context` object; it is
/// returned **unchanged** rather than silently discarded into a bare, empty
/// envelope.
pub fn frame_passport(passport: Value) -> Value {
    match passport {
        Value::Object(passport_map) => {
            let mut framed = passport_context();
            if let Value::Object(ref mut ctx_map) = framed {
                ctx_map.extend(passport_map);
            }
            framed
        }
        other => other,
    }
}

/// Extract the plain data from a JSON-LD framed passport (strip `@context`).
pub fn strip_context(framed: Value) -> Value {
    match framed {
        Value::Object(mut map) => {
            map.remove("@context");
            Value::Object(map)
        }
        other => other,
    }
}
