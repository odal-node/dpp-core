//! Issuance — turning a passport payload into an SD-JWT VC.

use serde_json::{Map, Value};

use dpp_crypto::keystore::KeyStore;
use dpp_crypto::sd_jwt::{Disclosure, SdJwt, build_payload, conceal};
use dpp_domain::Disclosure as DisclosureClass;
use dpp_domain::access::{DocumentScope, ProductGroupAccessPolicy};

use super::TYP;
use super::error::SdJwtVcError;

/// The envelope key below which a product group's own schema governs.
///
/// The same boundary the access filter draws, and for the same reason: a
/// product group's classes apply to its payload and stop there, while envelope
/// classes apply everywhere.
const PRODUCT_GROUP_DATA: &str = "productGroupData";

/// Claims that must never be concealed, however the disclosure policy classes
/// them.
///
/// Two reasons, and the second is a `MUST NOT`.
///
/// **Structural.** draft-ietf-oauth-sd-jwt-vc-19 clause 2.2.2 requires `iss` and
/// `vct` in the Unsecured Payload; `_sd_alg` is how a reader knows which hash to
/// recompute. A credential that concealed `vct` could not say what it is.
///
/// **Validity-controlling.** RFC 9901 clause 9.7: *"An Issuer MUST NOT allow any
/// content to be selectively disclosable that is critical for evaluating the
/// SD-JWT's authenticity or validity"*, and it names `iss`, `aud`, `exp`, `nbf`
/// and `cnf`. A concealable `exp` is one the holder can simply decline to
/// present, turning an expired credential into an unbounded one — the party the
/// expiry constrains is exactly the party choosing what to reveal.
///
/// These are listed rather than left to the policy because the policy's default
/// is `Public`, so today they survive by *accident*: nothing in a product
/// group's schema names them, so nothing classes them, so nothing conceals them.
/// That is not a guarantee — it is the absence of a counterexample, and it would
/// end the day a schema declared one of these names or the default changed.
const NEVER_CONCEALED: [&str; 8] = ["iss", "vct", "iat", "_sd_alg", "aud", "exp", "nbf", "cnf"];

/// Build and sign an SD-JWT VC over `payload`.
///
/// Every claim the policy classifies as anything other than
/// [`DisclosureClass::Public`] is concealed — at every depth, in both scopes.
/// Public claims travel in cleartext, which is correct and is the point: the
/// public view of a passport is public, and hiding it behind a disclosure would
/// make an anonymous read require a holder to cooperate.
///
/// # Why "non-public" and not "per audience"
///
/// A credential is issued once and presented many times, to audiences that are
/// not known at issuance and whose definitions are not settled in law. Grouping
/// disclosures by audience at issuance would bake today's audience taxonomy into
/// an artefact that has to outlive it. Concealing by *class* leaves the
/// audience decision where it belongs — at presentation — and is what makes a
/// later reclassification cost nothing.
///
/// # Errors
///
/// [`SdJwtVcError::PayloadNotAnObject`] if `payload` is not a JSON object, and
/// [`SdJwtVcError::Signing`] if the key store cannot sign.
pub fn issue(
    store: &KeyStore,
    key_id: &str,
    payload: &Value,
    policy: &ProductGroupAccessPolicy,
    issuer: &str,
    vct: &str,
    issued_at: i64,
) -> Result<SdJwt, SdJwtVcError> {
    let Some(object) = payload.as_object() else {
        return Err(SdJwtVcError::PayloadNotAnObject);
    };

    let mut claims = Map::new();
    // Clause 2.2.2: `iss` identifies the issuer and, being an HTTPS URI, selects
    // the JWT VC Issuer Metadata key-discovery mechanism of clause 2.5. Set
    // before the walk so a passport field named `iss` cannot displace it.
    claims.insert("iss".to_owned(), Value::String(issuer.to_owned()));
    claims.insert("vct".to_owned(), Value::String(vct.to_owned()));
    claims.insert("iat".to_owned(), Value::Number(issued_at.into()));

    let mut disclosures = Vec::new();
    let mut segments: Vec<String> = Vec::new();
    let concealed = conceal_object(
        object,
        policy,
        DocumentScope::Envelope,
        &mut segments,
        &mut disclosures,
    )?;
    for (key, value) in concealed {
        claims.entry(key).or_insert(value);
    }

    let jwt_payload = build_payload(claims, !disclosures.is_empty());
    let jwt = dpp_crypto::jws::sign_typed(store, key_id, &jwt_payload, Some(TYP))
        .map_err(|e| SdJwtVcError::Signing(e.to_string()))?;

    Ok(SdJwt::new(jwt, disclosures))
}

/// Conceal one object's non-public members, then descend into what remains.
///
/// A concealed member's whole value travels inside its disclosure, subtree and
/// all, so there is nothing left to descend into for that branch — hiding a
/// field hides everything under it, which is the same containment the access
/// filter gives by dropping the branch.
fn conceal_object(
    object: &Map<String, Value>,
    policy: &ProductGroupAccessPolicy,
    scope: DocumentScope,
    segments: &mut Vec<String>,
    disclosures: &mut Vec<Disclosure>,
) -> Result<Map<String, Value>, SdJwtVcError> {
    // Classify first, so the predicate handed to `conceal` is a lookup rather
    // than a policy call — `conceal` decides nothing about disclosure, and this
    // keeps it that way.
    let mut hide = Vec::new();
    for key in object.keys() {
        // 🚨 Root only. These are registered claims of the *token*, and the
        // reason they are exempt — a verifier reads them before any disclosure
        // is processed — is true of the payload root and nowhere else. Applied
        // at every depth, as it was, the list silently exempted any nested
        // field that happened to share one of these eight names: a
        // `productGroupData` member called `exp` or `aud` would travel in
        // cleartext however the policy classified it, which is precisely the
        // over-disclosure this design exists to prevent.
        if segments.is_empty() && NEVER_CONCEALED.contains(&key.as_str()) {
            continue;
        }
        segments.push(key.clone());
        let borrowed: Vec<&str> = segments.iter().map(String::as_str).collect();
        let class = policy.disclosure_for_path(&borrowed, scope);
        drop(borrowed);
        segments.pop();
        if class != DisclosureClass::Public {
            hide.push(key.clone());
        }
    }

    let (mut kept, mut produced) = conceal(object, |name| hide.iter().any(|h| h == name))?;
    disclosures.append(&mut produced);

    // Descend into the members that stayed in cleartext.
    for (key, value) in kept.iter_mut() {
        if key == "_sd" {
            continue;
        }
        // The key is classified in the scope it sits in; its children in the
        // scope it opens. Crossing at the child keeps `productGroupData` an
        // envelope field, so a product group's schema cannot reclassify the
        // container it is carried in.
        let child_scope = if scope == DocumentScope::ProductGroupData || key == PRODUCT_GROUP_DATA {
            DocumentScope::ProductGroupData
        } else {
            DocumentScope::Envelope
        };
        segments.push(key.clone());
        descend(value, policy, child_scope, segments, disclosures)?;
        segments.pop();
    }

    Ok(kept)
}

/// Walk into a value that stayed in cleartext.
///
/// Arrays add no classification segment and do not change scope — an element
/// sits exactly where its key sits — which mirrors the access filter's walk so
/// the two cannot disagree about where a class applies.
fn descend(
    value: &mut Value,
    policy: &ProductGroupAccessPolicy,
    scope: DocumentScope,
    segments: &mut Vec<String>,
    disclosures: &mut Vec<Disclosure>,
) -> Result<(), SdJwtVcError> {
    match value {
        Value::Object(map) => {
            *map = conceal_object(map, policy, scope, segments, disclosures)?;
        }
        Value::Array(items) => {
            for item in items {
                descend(item, policy, scope, segments, disclosures)?;
            }
        }
        _ => {}
    }
    Ok(())
}
