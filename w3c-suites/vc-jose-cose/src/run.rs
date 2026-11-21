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
use dpp_crypto::sd_jwt::ClaimPath;
use dpp_vc::jose::{
    KeyBindingRequirement, VerifyOptions, sign_credential, sign_credential_sd_jwt,
    sign_presentation, sign_presentation_sd_jwt, verify_credential, verify_credential_sd_jwt,
    verify_presentation, verify_presentation_sd_jwt,
};

use crate::feature::Feature;

/// The library's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// It signed, with this compact JWS or SD-JWT, or it verified (`None`).
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
///
/// `disclose` is the claims an SD-JWT conceals, which the suite gives to rows 17 to 19 as
/// `--sd`. A JOSE feature given any is not a verdict: the suite never asks for it, and
/// signing without concealing what was asked to be concealed would be an answer to another
/// question. `judge` refuses `--sd` on a JOSE feature even when the list is empty, before
/// this is reached.
pub fn issue(
    feature: Feature,
    document: &str,
    key_text: &str,
    disclose: &[ClaimPath],
) -> Result<Verdict, String> {
    if !feature.is_sd_jwt() && !disclose.is_empty() {
        return Err("--sd applies to the SD-JWT features only".to_owned());
    }
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
        Feature::CredentialSdJwt => {
            sign_credential_sd_jwt(&document, &key, kid, &[], disclose, None).map(|t| t.serialise())
        }
        Feature::PresentationSdJwt => {
            sign_presentation_sd_jwt(&document, &key, kid, &[], disclose, None)
                .map(|t| t.serialise())
        }
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
/// The credentials inside are not verified. A `vp+sd-jwt` is verified the same way, with
/// `verify_presentation_sd_jwt`.
///
/// No time is passed, so `exp` and `nbf` are not read. The suite's
/// presentations carry an `exp` in December 2024, so under the clock row 7 would be
/// refused for its age, and rows 14 and 16, which expect a refusal for an envelope,
/// would be refused for their age instead and pass for the wrong reason. The suite
/// states no verification time.
///
/// No audience is passed either, and Key Binding is not required: none of the suite's
/// inputs carries `aud`, `cnf` or a Key Binding JWT, and the suite names no verifier.
pub fn verify(feature: Feature, token_text: &str, key_text: &str) -> Result<Verdict, String> {
    let key_file = key_file(key_text)?;
    let key = PublicKey::from_jwk(&key_file["publicKeyJwk"])
        .ok_or("the key file's publicKeyJwk is not a usable public key")?;
    // Some of the suite's token files end in a newline, which is no part of the token.
    let token = token_text.trim();

    let options = VerifyOptions::new();
    let binding = KeyBindingRequirement::NotRequired;

    Ok(match feature {
        Feature::CredentialJose => verify_credential(token, &key, options).map(|_| ()),
        Feature::PresentationJose => verify_presentation(token, &key, options).map(|_| ()),
        Feature::CredentialSdJwt => {
            verify_credential_sd_jwt(token, &key, options, &binding).map(|_| ())
        }
        Feature::PresentationSdJwt => {
            verify_presentation_sd_jwt(token, &key, options, &binding).map(|_| ())
        }
    }
    .map_or_else(
        |refusal| Verdict::Failure(refusal.to_string()),
        |()| Verdict::Success(None),
    ))
}
