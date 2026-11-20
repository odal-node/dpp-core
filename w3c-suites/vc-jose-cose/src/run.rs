//! What the suite's two commands do with `dpp-vc`, and the verdict each reports.
//!
//! Two kinds of outcome, kept apart. A **verdict** is the library answering: it
//! signed, it verified, or it refused, and a refusal is the suite's `failure`. An
//! **unusable input** is the harness being unable to ask: a key file that is not a
//! key, a document that is not JSON. That is not a verdict, so it is an `Err` here,
//! and no output file is written for it: the suite scores a missing file as an
//! error, where a `failure` written for it would let a broken run pass every row
//! that expects one.

use serde_json::{Value, json};

use dpp_crypto::jws::PublicKey;
use dpp_crypto::keystore::KeyEntry;
use dpp_vc::jose::{sign_credential, sign_presentation, verify_credential, verify_presentation};

use crate::feature::Feature;

/// The library's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// It signed, with this compact JWS, or it verified (`None`).
    Success(Option<String>),
    /// It refused, and this is why.
    Failure(String),
}

impl Verdict {
    /// The JSON `tests/test-util.js` reads: `result` is `success` or `failure`, and
    /// `data` carries an issued token. A refusal's reason rides in `reason`.
    pub fn to_json(&self) -> Value {
        match self {
            Self::Success(Some(token)) => json!({"result": "success", "data": token}),
            Self::Success(None) => json!({"result": "success"}),
            Self::Failure(reason) => json!({"result": "failure", "reason": reason}),
        }
    }
}

fn key_file(text: &str) -> Result<Value, String> {
    serde_json::from_str(text).map_err(|e| format!("the key file is not JSON: {e}"))
}

/// Issue `document` (JSON text) under the `secretKeyJwk` of `key_text`.
///
/// The `kid` is the key file's `id`: the absolute URL of its verification method,
/// which VC-JOSE-COSE §4.2 asks for when the document has no `iss`. No extension
/// term is declared, so a document with a member outside the VC Data Model 2.0
/// vocabulary is refused, which is what the suite's row 5 expects.
pub fn issue(feature: Feature, document: &str, key_text: &str) -> Result<Verdict, String> {
    let document: Value =
        serde_json::from_str(document).map_err(|e| format!("the input is not JSON: {e}"))?;
    let key_file = key_file(key_text)?;
    let kid = key_file["id"]
        .as_str()
        .ok_or("the key file has no string `id`")?;
    let key = KeyEntry::from_private_jwk(&key_file["secretKeyJwk"])
        .map_err(|e| format!("the key file's secretKeyJwk is not a usable private key: {e}"))?;

    let signed = match feature {
        Feature::CredentialJose => sign_credential(&document, &key, kid, &[]),
        Feature::PresentationJose => sign_presentation(&document, &key, kid, &[]),
    };
    Ok(match signed {
        Ok(token) => Verdict::Success(Some(token)),
        Err(refusal) => Verdict::Failure(refusal.to_string()),
    })
}

/// Verify `token_text` under the `publicKeyJwk` of `key_text`.
///
/// A presentation is verified with `verify_presentation` and nothing more:
/// VC-JOSE-COSE §5.1 defines verifying a presentation as validating its own JWT, and
/// the suite's row 7 envelops a `vc+sd-jwt` and a `vc+cose` it expects to succeed.
/// The credentials inside are not verified.
///
/// No time is passed (`now` is `None`), so `exp` and `nbf` are not read. The suite's
/// presentations carry an `exp` in December 2024, so under the clock row 7 would be
/// refused for its age, and rows 14 and 16, which expect a refusal for an envelope,
/// would be refused for their age instead and pass for the wrong reason. The suite
/// states no verification time.
pub fn verify(feature: Feature, token_text: &str, key_text: &str) -> Result<Verdict, String> {
    let key_file = key_file(key_text)?;
    let key = PublicKey::from_jwk(&key_file["publicKeyJwk"])
        .ok_or("the key file's publicKeyJwk is not a usable public key")?;
    // Some of the suite's token files end in a newline, which is no part of the token.
    let token = token_text.trim();

    Ok(match feature {
        Feature::CredentialJose => verify_credential(token, &key, None).map(|_| ()),
        Feature::PresentationJose => verify_presentation(token, &key, None).map(|_| ()),
    }
    .map_or_else(
        |refusal| Verdict::Failure(refusal.to_string()),
        |()| Verdict::Success(None),
    ))
}
