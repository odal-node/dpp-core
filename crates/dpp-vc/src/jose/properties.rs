//! The properties a credential or presentation must carry, checked when it is signed
//! and again when it is verified.
//!
//! VC Data Model 2.0 §1.3 says a conforming verifier "MUST check that each required
//! property satisfies the normative requirements for that property, and MUST produce
//! errors when non-conforming documents are detected". The signer is held to the same
//! rules, so a token this crate would refuse is not produced.
//!
//! Nothing is expanded. Each rule is read off the document's JSON: whether `type` names
//! `VerifiableCredential`, whether `issuer` is a URL, and so on.

use serde_json::{Map, Value};

use super::absolute_url::is_absolute_url;
use super::error::JoseError;

/// The one URL the first item of `@context` must be (VC DM 2.0 §4.3).
const V2_CONTEXT: &str = "https://www.w3.org/ns/credentials/v2";

/// `@context`, as VC DM 2.0 §4.3 states it: "an ordered set where the first item is a
/// URL with the value https://www.w3.org/ns/credentials/v2. Subsequent items in the
/// ordered set MUST be composed of any combination of URLs and objects".
///
/// A single string equal to that URL is the same one-item set in JSON-LD, and is
/// accepted. Anything else is refused: absent, neither a string nor an array, a single
/// string of another URL, an empty array, an array whose first item is not that URL,
/// and a later item that is neither a URL nor an object.
pub(crate) fn check_context(document: &Map<String, Value>) -> Result<(), JoseError> {
    let refuse = |reason: &str| JoseError::InvalidContext {
        reason: reason.to_owned(),
    };
    let items = match document.get("@context") {
        None => return Err(refuse("is absent")),
        Some(Value::String(url)) if url == V2_CONTEXT => return Ok(()),
        Some(Value::String(_)) => {
            return Err(refuse(
                "is a single string other than https://www.w3.org/ns/credentials/v2",
            ));
        }
        Some(Value::Array(items)) => items,
        Some(_) => return Err(refuse("is neither a string nor an array")),
    };
    let Some((first, later)) = items.split_first() else {
        return Err(refuse("is an empty array"));
    };
    if first.as_str() != Some(V2_CONTEXT) {
        return Err(refuse(
            "does not begin with https://www.w3.org/ns/credentials/v2",
        ));
    }
    for (position, item) in later.iter().enumerate() {
        let well_formed = match item {
            Value::String(url) => is_absolute_url(url),
            Value::Object(_) => true,
            _ => false,
        };
        if !well_formed {
            return Err(refuse(&format!(
                "item {} is neither a URL nor an object",
                position + 2
            )));
        }
    }
    Ok(())
}

/// The required properties of a credential: `type` names `VerifiableCredential` (VC DM
/// 2.0 §4.5), `issuer` is a URL or an object with a URL `id` (§4.7), and
/// `credentialSubject` is an object or a non-empty array of objects (§4.8).
pub(crate) fn check_credential_properties(document: &Map<String, Value>) -> Result<(), JoseError> {
    check_type(document, "VerifiableCredential")?;
    check_issuer_property(document)?;
    check_credential_subject(document)
}

/// The required property of a presentation: `type` names `VerifiablePresentation` (VC
/// DM 2.0 §4.13). Its `@context` is checked by [`check_context`].
pub(crate) fn check_presentation_properties(
    document: &Map<String, Value>,
) -> Result<(), JoseError> {
    check_type(document, "VerifiablePresentation")
}

/// `holder`, when present, is a URL or an object whose `id` is a URL (VC DM 2.0 §4.13).
/// A presentation with no `holder` is valid.
pub(crate) fn check_holder(document: &Map<String, Value>) -> Result<(), JoseError> {
    match document.get("holder") {
        None => Ok(()),
        Some(holder) if id_of(holder).is_some_and(is_absolute_url) => Ok(()),
        Some(_) => Err(JoseError::InvalidHolder {
            reason: "is neither a URL nor an object with a URL id".to_owned(),
        }),
    }
}

/// The URL that an `issuer` or a `holder` value names: the value itself when it is a
/// string, or its `id` when it is an object. Whether that is an absolute URL is for the
/// caller to test.
pub(crate) fn id_of(value: &Value) -> Option<&str> {
    match value {
        Value::String(id) => Some(id),
        Value::Object(object) => object.get("id")?.as_str(),
        _ => None,
    }
}

/// `type` is a string or an array of strings, with at least one term, and `expected`
/// is one of its terms (VC DM 2.0 §4.5, §4.13).
fn check_type(document: &Map<String, Value>, expected: &str) -> Result<(), JoseError> {
    let refuse = |reason: String| JoseError::InvalidType { reason };
    let terms: Vec<&str> = match document.get("type") {
        None => return Err(refuse("is absent".to_owned())),
        Some(Value::String(term)) => vec![term.as_str()],
        Some(Value::Array(items)) => items
            .iter()
            .map(Value::as_str)
            .collect::<Option<Vec<&str>>>()
            .ok_or_else(|| refuse("is neither a string nor an array of strings".to_owned()))?,
        Some(_) => {
            return Err(refuse(
                "is neither a string nor an array of strings".to_owned(),
            ));
        }
    };
    if terms.is_empty() {
        return Err(refuse("is an empty array".to_owned()));
    }
    if terms.contains(&expected) {
        Ok(())
    } else {
        Err(refuse(format!("does not include {expected}")))
    }
}

/// `issuer` is a URL, or an object whose `id` is a URL (VC DM 2.0 §4.7).
fn check_issuer_property(document: &Map<String, Value>) -> Result<(), JoseError> {
    match document.get("issuer") {
        None => Err(JoseError::InvalidIssuer {
            reason: "is absent".to_owned(),
        }),
        Some(issuer) if id_of(issuer).is_some_and(is_absolute_url) => Ok(()),
        Some(_) => Err(JoseError::InvalidIssuer {
            reason: "is neither a URL nor an object with a URL id".to_owned(),
        }),
    }
}

/// `credentialSubject` is an object, or a non-empty array of objects, and each object is
/// the subject of one or more claims besides its `id`. A credential contains claims about
/// one or more subjects, and each subject object MUST be the subject of one or more
/// claims, which MAY be accompanied by an `id` (VC DM 2.0 §4.8). An object whose only
/// member is `id` carries no claim, so it is refused.
fn check_credential_subject(document: &Map<String, Value>) -> Result<(), JoseError> {
    let refuse = |reason: &str| JoseError::InvalidCredentialSubject {
        reason: reason.to_owned(),
    };
    let subjects: Vec<&Value> = match document.get("credentialSubject") {
        None => return Err(refuse("is absent")),
        Some(subject @ Value::Object(_)) => vec![subject],
        Some(Value::Array(items)) if items.is_empty() => {
            return Err(refuse("is an empty array"));
        }
        Some(Value::Array(items)) => items.iter().collect(),
        Some(_) => {
            return Err(refuse(
                "is neither an object nor a non-empty array of objects",
            ));
        }
    };
    if !subjects.iter().all(|subject| subject.is_object()) {
        return Err(refuse(
            "is neither an object nor a non-empty array of objects",
        ));
    }
    let claimed = |subject: &&Value| {
        subject
            .as_object()
            .is_some_and(|object| object.keys().any(|key| key != "id"))
    };
    if subjects.iter().all(claimed) {
        Ok(())
    } else {
        Err(refuse("has an object with no claim besides its id"))
    }
}
