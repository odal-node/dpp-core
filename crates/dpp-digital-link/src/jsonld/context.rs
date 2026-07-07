//! JSON-LD context envelope: build / frame / strip a DPP passport payload.

use serde_json::{Value, json};

/// Build a minimal JSON-LD context for an Odal Node passport.
///
/// Suitable for embedding in or linking from a DPP payload to assert
/// semantic interoperability with GS1, Schema.org, and EU ESPR vocabularies.
pub fn passport_context() -> Value {
    json!({
        "@context": [
            "https://www.w3.org/ns/did/v1",
            "https://ref.gs1.org/standards/digital-link/context/",
            {
                "dpp": "https://schema.odal-node.io/dpp#",
                "gs1": "https://ref.gs1.org/voc/",
                "schema": "https://schema.org/",
                "gtin": "gs1:gtin",
                "sector": "dpp:sector",
                "passportId": "dpp:passportId",
                "status": "dpp:status",
                "sectorData": "dpp:sectorData",
                "complianceResult": "dpp:complianceResult",
                "createdAt": "schema:dateCreated",
                "updatedAt": "schema:dateModified",
                "jws": "dpp:jws"
            }
        ]
    })
}

/// Wrap a passport JSON value in a JSON-LD envelope.
pub fn frame_passport(passport: Value) -> Value {
    let mut framed = passport_context();
    if let Value::Object(ref mut ctx_map) = framed
        && let Value::Object(passport_map) = passport
    {
        ctx_map.extend(passport_map);
    }
    framed
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
