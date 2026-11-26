//! A credential and a presentation round-trip under every algorithm: EdDSA, ES256,
//! ES384 and ES512.

use base64::Engine as _;
use serde_json::Value;

use super::super::VerifyOptions;
use super::super::{
    JoseError, sign_credential, sign_presentation, verify_credential, verify_presentation,
};
use crate::test_support::EVERY_ALGORITHM;
use crate::test_support::{KID, credential, generated, presentation};

fn header_of(token: &str) -> Value {
    let segment = token.split('.').next().expect("a header");
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(segment)
        .expect("base64url");
    serde_json::from_slice(&bytes).expect("JSON")
}

#[test]
fn a_credential_round_trips_under_every_algorithm() {
    let expected_alg = ["EdDSA", "ES256", "ES384", "ES512"];
    for (algorithm, alg) in EVERY_ALGORITHM.into_iter().zip(expected_alg) {
        let key = generated(algorithm);

        let token = sign_credential(&credential(), &key, KID, &[]).expect("signs");

        let header = header_of(&token);
        assert_eq!(header["alg"], alg);
        assert_eq!(header["typ"], "vc+jwt");
        assert_eq!(header["cty"], "vc");
        assert_eq!(header["kid"], KID);
        let verified =
            verify_credential(&token, &key.public_key(), VerifyOptions::new()).expect("verifies");
        assert_eq!(verified, credential(), "{alg}");
    }
}

#[test]
fn a_presentation_round_trips_under_every_algorithm() {
    for algorithm in EVERY_ALGORITHM {
        let key = generated(algorithm);
        let holder = presentation(Vec::new());

        let token = sign_presentation(&holder, &key, KID, &[]).expect("signs");

        let header = header_of(&token);
        assert_eq!(header["typ"], "vp+jwt");
        assert_eq!(header["cty"], "vp");
        let verified =
            verify_presentation(&token, &key.public_key(), VerifyOptions::new()).expect("verifies");
        assert_eq!(verified.payload(), &holder, "{algorithm:?}");
    }
}

/// The key decides the algorithm, so a token signed under one algorithm is refused
/// under a key of any other, and says which.
#[test]
fn a_key_of_another_algorithm_refuses_the_token() {
    for signer in EVERY_ALGORITHM {
        let token = sign_credential(&credential(), &generated(signer), KID, &[]).expect("signs");
        for verifier in EVERY_ALGORITHM {
            if verifier == signer {
                continue;
            }
            let refused = verify_credential(
                &token,
                &generated(verifier).public_key(),
                VerifyOptions::new(),
            )
            .expect_err("refused");
            assert_eq!(
                refused,
                JoseError::AlgorithmMismatch {
                    header_alg: Some(signer.jose_alg().to_owned()),
                    key_alg: verifier,
                }
            );
        }
    }
}
