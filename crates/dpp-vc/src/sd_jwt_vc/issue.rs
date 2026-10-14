//! Issuance — turning a passport payload into an SD-JWT VC.

use serde_json::{Map, Value};

use dpp_crypto::keystore::KeyStore;
use dpp_crypto::sd_jwt::{Disclosure, SdJwt, build_payload, conceal, holder_key_confirmation};
use dpp_domain::Disclosure as DisclosureClass;
use dpp_domain::access::{DocumentScope, ProductGroupAccessPolicy};

use super::error::SdJwtVcError;
use super::{PASSPORT_CLAIM, TYP};

/// The envelope key below which a product group's own schema governs.
///
/// The same boundary the access filter draws, and for the same reason: a
/// product group's classes apply to its payload and stop there, while envelope
/// classes apply everywhere.
const PRODUCT_GROUP_DATA: &str = "productGroupData";

/// What the Issuer says about the credential itself, as opposed to the passport it
/// carries: the claims [`issue`] sets at the root, beside the passport's
/// [`PASSPORT_CLAIM`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Issuance<'a> {
    /// `iss`. Being an HTTPS URI, it selects the JWT VC Issuer Metadata key
    /// discovery of clause 2.5.
    pub issuer: &'a str,
    /// `vct`, the type of the credential (clause 2.2.2.1).
    pub vct: &'a str,
    /// `iat`, in seconds since the epoch.
    pub issued_at: i64,
    /// The Holder's Ed25519 public key, to bind the credential to.
    ///
    /// Written as `cnf` with RFC 7800's `jwk` confirmation method, which clause
    /// 2.2.2.3 recommends, and which Key Binding requires: a presentation must then
    /// carry a Key Binding JWT signed by this key. `None` issues a credential that
    /// anyone holding a copy can present as their own (RFC 9901 clause 9.5).
    pub holder_key: Option<[u8; 32]>,
}

/// Build and sign an SD-JWT VC over `payload`.
///
/// The passport sits under one claim, [`PASSPORT_CLAIM`]. The root holds the
/// claims the Issuer sets (`iss`, `vct`, `iat`, and `cnf` when a Holder key is
/// named) and nothing else, so no passport member can be read as a registered
/// claim of the credential. The passport has a lifecycle `status`; at the root
/// that name is the profile's reference to a Token Status List.
///
/// Every claim of the passport the policy classifies as anything other than
/// [`DisclosureClass::Public`] is concealed — at every depth, in both scopes.
/// Public claims travel in cleartext, which is correct and is the point: the
/// public view of a passport is public, and hiding it behind a disclosure would
/// make an anonymous read require a holder to cooperate.
///
/// # The claims the Issuer sets are never concealed
///
/// RFC 9901 clause 9.7: *"An Issuer MUST NOT allow any content to be selectively
/// disclosable that is critical for evaluating the SD-JWT's authenticity or
/// validity"*, and it names `iss`, `aud`, `exp`, `nbf` and `cnf`. A concealable
/// `exp` is one the Holder can decline to present, turning an expired credential
/// into an unbounded one. Here that holds by construction: the root claims are
/// written directly and only the passport is walked for concealment, so no policy,
/// and no schema, can reach them. `_sd_alg` is added by [`build_payload`] when
/// there is anything to conceal.
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
    credential: &Issuance<'_>,
) -> Result<SdJwt, SdJwtVcError> {
    let Some(object) = payload.as_object() else {
        return Err(SdJwtVcError::PayloadNotAnObject);
    };

    let mut claims = Map::new();
    // Clause 2.2.2: `iss` identifies the issuer and, being an HTTPS URI, selects
    // the JWT VC Issuer Metadata key-discovery mechanism of clause 2.5.
    claims.insert(
        "iss".to_owned(),
        Value::String(credential.issuer.to_owned()),
    );
    claims.insert("vct".to_owned(), Value::String(credential.vct.to_owned()));
    claims.insert("iat".to_owned(), Value::Number(credential.issued_at.into()));
    // Clause 2.2.2.3: REQUIRED when Key Binding is to be supported, and never in a
    // Disclosure.
    if let Some(key) = credential.holder_key {
        claims.insert("cnf".to_owned(), holder_key_confirmation(&key));
    }

    let mut disclosures = Vec::new();
    let mut segments: Vec<String> = Vec::new();
    let concealed = conceal_object(
        object,
        policy,
        DocumentScope::Envelope,
        &mut segments,
        &mut disclosures,
    )?;
    claims.insert(PASSPORT_CLAIM.to_owned(), Value::Object(concealed));

    let jwt_payload = build_payload(claims, !disclosures.is_empty());
    // The `kid` is the key's thumbprint URI, which is the `kid` of its entry in
    // the issuer metadata's key set (clause 4.2) and not the DID URL a
    // `did:web` verifier would use: this profile names no DID method.
    let kid = store
        .public_key(key_id)
        .ok_or_else(|| SdJwtVcError::Signing(format!("no key found for {key_id}")))?
        .thumbprint_uri()
        .map_err(|e| SdJwtVcError::Signing(e.to_string()))?;
    let jwt = dpp_crypto::jws::sign_typed(store, key_id, &jwt_payload, &kid, Some(TYP), None)
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
