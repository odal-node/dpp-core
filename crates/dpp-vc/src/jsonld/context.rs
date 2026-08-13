//! JSON-LD context envelope: build / frame / strip a DPP passport payload.

use std::sync::OnceLock;

use serde_json::{Value, json};

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
/// **`gtin` is back, and only `gtin`.** GS1's own definition was read on
/// 2026-08-11 at `https://ref.gs1.org/voc/gtin`: the 14-digit key, `xsd:string`,
/// `skos:exactMatch` to `schema:gtin14` and `closeMatch` to the shorter forms.
/// That is [`Gtin`](dpp_domain::Gtin)'s shape exactly — 14 digits, mod-10 check,
/// shorter forms zero-padded to the canonical one — so the term now says
/// something true. The vocabulary is Apache-2.0, which our own licence admits
/// without a copyleft obligation.
///
/// The other two stay `dpp:`. Schema.org is `tracked` in `dpp-vocab`: evaluated,
/// not adopted. And note that what was read here was **one term**, not the GS1
/// vocabulary — declaring the `gs1:` prefix is what makes the compact form
/// expand, not a claim that anything else under it has been checked.
///
/// The literal is built once and cloned per call — callers extend the
/// returned value (e.g. [`frame_passport`] merges passport fields into it),
/// so it must stay an owned, independently-mutable `Value` per call site.
pub fn passport_context() -> Value {
    static CONTEXT: OnceLock<Value> = OnceLock::new();
    CONTEXT
        .get_or_init(|| {
            json!({
                "@context": [
                    REMOTE_CONTEXTS[0],
                    {
                        "dpp": "https://schema.odal-node.io/dpp#",
                        "gs1": "https://ref.gs1.org/voc/",
                        "gtin": "gs1:gtin",
                        "sector": "dpp:sector",
                        "passportId": "dpp:passportId",
                        "status": "dpp:status",
                        "sectorData": "dpp:sectorData",
                        "complianceResult": "dpp:complianceResult",
                        "createdAt": "dpp:createdAt",
                        "updatedAt": "dpp:updatedAt",
                        "jws": "dpp:jws"
                    }
                ]
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
