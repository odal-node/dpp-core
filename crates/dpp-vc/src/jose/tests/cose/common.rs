//! Messages for the COSE tests, written byte by byte so a test can build the message the
//! signer would never write, with a genuine signature over it. A refusal is then provably the
//! rule it names and not a signature that never verified.

use dpp_crypto::cose::cose_alg;
use dpp_crypto::keystore::KeyEntry;
use serde_json::Value;

use crate::jose::{JoseError, VerifyOptions, verify_credential_cose};
use crate::test_support::{KID, public_key, signing_key};

fn head(major: u8, argument: u64) -> Vec<u8> {
    let major = major << 5;
    if argument < 24 {
        vec![major | u8::try_from(argument).expect("below 24")]
    } else if let Ok(argument) = u8::try_from(argument) {
        vec![major | 24, argument]
    } else if let Ok(argument) = u16::try_from(argument) {
        [&[major | 25][..], &argument.to_be_bytes()].concat()
    } else {
        [
            &[major | 26][..],
            &u32::try_from(argument).expect("fits").to_be_bytes(),
        ]
        .concat()
    }
}

pub(super) fn uint(value: u64) -> Vec<u8> {
    head(0, value)
}

pub(super) fn nint(value: i64) -> Vec<u8> {
    head(1, u64::try_from(-1 - value).expect("negative"))
}

pub(super) fn bstr(bytes: &[u8]) -> Vec<u8> {
    [head(2, bytes.len() as u64), bytes.to_vec()].concat()
}

pub(super) fn tstr(text: &str) -> Vec<u8> {
    [head(3, text.len() as u64), text.as_bytes().to_vec()].concat()
}

pub(super) fn array(items: &[Vec<u8>]) -> Vec<u8> {
    [head(4, items.len() as u64), items.concat()].concat()
}

pub(super) fn map(pairs: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let body: Vec<u8> = pairs
        .iter()
        .flat_map(|(k, v)| [k.clone(), v.clone()].concat())
        .collect();
    [head(5, pairs.len() as u64), body].concat()
}

/// The pieces of a protected header. A `None` leaves the parameter out.
#[derive(Clone)]
pub(super) struct Header {
    pub(super) typ: Option<Vec<u8>>,
    pub(super) cty: Option<Vec<u8>>,
    pub(super) kid: Option<Vec<u8>>,
    /// The pairs of the CWT Claims header parameter (15), already encoded.
    pub(super) cwt: Option<Vec<(Vec<u8>, Vec<u8>)>>,
}

impl Header {
    /// The header the signer writes for a credential, without CWT claims.
    pub(super) fn credential(kid: &str) -> Self {
        Self {
            typ: Some(tstr("application/vc+cose")),
            cty: Some(tstr("application/vc")),
            kid: Some(bstr(kid.as_bytes())),
            cwt: None,
        }
    }

    /// The header the signer writes for a presentation, without CWT claims.
    pub(super) fn presentation(kid: &str) -> Self {
        Self {
            typ: Some(tstr("application/vp+cose")),
            cty: Some(tstr("application/vp")),
            ..Self::credential(kid)
        }
    }

    pub(super) fn typ(mut self, text: &str) -> Self {
        self.typ = Some(tstr(text));
        self
    }

    pub(super) fn cty(mut self, text: &str) -> Self {
        self.cty = Some(tstr(text));
        self
    }

    pub(super) fn without_typ(mut self) -> Self {
        self.typ = None;
        self
    }

    pub(super) fn without_cty(mut self) -> Self {
        self.cty = None;
        self
    }

    pub(super) fn cwt(mut self, claims: Vec<(Vec<u8>, Vec<u8>)>) -> Self {
        self.cwt = Some(claims);
        self
    }

    fn protected(&self, key: &KeyEntry) -> Vec<u8> {
        let mut pairs = vec![(uint(1), nint(cose_alg(key.algorithm())))];
        if let Some(cty) = &self.cty {
            pairs.push((uint(3), cty.clone()));
        }
        if let Some(kid) = &self.kid {
            pairs.push((uint(4), kid.clone()));
        }
        if let Some(claims) = &self.cwt {
            pairs.push((uint(15), map(claims)));
        }
        if let Some(typ) = &self.typ {
            pairs.push((uint(16), typ.clone()));
        }
        map(&pairs)
    }
}

/// A tagged `COSE_Sign1` of `payload`, signed by `key` over the `Sig_structure` of its
/// protected header, with an empty unprotected header.
pub(super) fn forge(key: &KeyEntry, header: &Header, payload: &[u8]) -> Vec<u8> {
    forge_with_unprotected(key, header, &map(&[]), payload)
}

/// [`forge`] with the given encoded map as the unprotected header.
pub(super) fn forge_with_unprotected(
    key: &KeyEntry,
    header: &Header,
    unprotected: &[u8],
    payload: &[u8],
) -> Vec<u8> {
    let protected = header.protected(key);
    let to_be_signed = array(&[
        tstr("Signature1"),
        bstr(&protected),
        bstr(&[]),
        bstr(payload),
    ]);
    let signature = key.sign(&to_be_signed).expect("an unrevoked key signs");
    [
        vec![0xd2],
        array(&[
            bstr(&protected),
            unprotected.to_vec(),
            bstr(payload),
            bstr(&signature),
        ]),
    ]
    .concat()
}

/// [`forge`] of the JSON text of `document`, in the order it has.
pub(super) fn forge_document(key: &KeyEntry, header: &Header, document: &Value) -> Vec<u8> {
    forge(
        key,
        header,
        &serde_json::to_vec(document).expect("serialises"),
    )
}

// ── The suite's keys and the verifiers the tests share ───────────────────────

pub(super) const ALGORITHMS: [&str; 4] = ["ed25519", "p256", "p384", "p521"];
pub(super) const THUMBPRINT: &str = "73voMXFNmNlOEpuYCSJlh8eN0dscykoO6gBukgRS1uU";
pub(super) const AUDIENCE: &str = "https://verifier.example";

pub(super) fn issuer() -> KeyEntry {
    signing_key("p256")
}

pub(super) fn holder() -> KeyEntry {
    signing_key("p384")
}

pub(super) fn cred_header() -> Header {
    Header::credential(KID)
}

/// The message of `document` under the issuer's key and `header`.
pub(super) fn forged_credential(document: &Value, header: &Header) -> Vec<u8> {
    forge_document(&issuer(), header, document)
}

pub(super) fn verify_vc(bytes: &[u8]) -> Result<Value, JoseError> {
    verify_credential_cose(bytes, &public_key("p256"), VerifyOptions::new())
}

pub(super) fn verify_vc_with(bytes: &[u8], options: VerifyOptions<'_>) -> Result<Value, JoseError> {
    verify_credential_cose(bytes, &public_key("p256"), options)
}

pub(super) fn cwt(pairs: Vec<(Vec<u8>, Vec<u8>)>) -> Header {
    cred_header().cwt(pairs)
}

/// One claim of a CWT Claims map: its encoded key and value.
pub(super) fn claim(key: u64, value: Vec<u8>) -> (Vec<u8>, Vec<u8>) {
    (uint(key), value)
}
