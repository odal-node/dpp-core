//! SD-JWT VC under every issuer and holder algorithm: issuance, Key Binding, the
//! audience a presentation is held to, and the issuer metadata a verifier reads the
//! issuer key from.

use base64::Engine;
use serde_json::{Value, json};

use dpp_crypto::jws::{KeyAlgorithm, PublicKey};
use dpp_crypto::keystore::{KeyEntry, KeyStore};
use dpp_crypto::sd_jwt::{KeyBinding, KeyBindingPolicy, SdJwtError, holder_key_confirmation};
use dpp_domain::ProductGroup;

use super::test_fixtures::*;
use super::{
    Issuance, PASSPORT_CLAIM, SdJwtVcError, build_issuer_metadata, issue, vct_for, verify,
};
use crate::test_support::{EVERY_ALGORITHM, temp_store, temp_store_with};

const AUDIENCE: &str = "https://verifier.example";
const NONCE: &str = "n-SdJwtVcAlg";

/// The EC curves, each with the JWK it is published under: `kty`, `crv` and `alg`.
const EC_SHAPES: [(KeyAlgorithm, &str, &str, &str); 3] = [
    (KeyAlgorithm::EcdsaP256, "EC", "P-256", "ES256"),
    (KeyAlgorithm::EcdsaP384, "EC", "P-384", "ES384"),
    (KeyAlgorithm::EcdsaP521, "EC", "P-521", "ES512"),
];

fn vct() -> String {
    vct_for(ProductGroup::Battery, SCHEMA_VERSION)
}

/// A Holder key on `algorithm`, as the Holder keeps it.
fn holder_key(algorithm: KeyAlgorithm) -> KeyEntry {
    temp_store_with("sd-jwt-vc-alg-holder", "holder", algorithm)
        .load_key("holder")
        .expect("the key was just generated")
}

fn binding() -> KeyBinding {
    KeyBinding {
        audience: AUDIENCE.to_owned(),
        nonce: NONCE.to_owned(),
        issued_at: ISSUED_AT + 60,
    }
}

/// The Verifier's policy for [`AUDIENCE`], at the instant [`now`] names.
fn key_binding_policy() -> KeyBindingPolicy {
    KeyBindingPolicy {
        audience: AUDIENCE.to_owned(),
        nonce: NONCE.to_owned(),
        now: ISSUED_AT + 60,
        max_age: 300,
        leeway: 5,
    }
}

/// The protected header of a compact JWT, read without checking anything.
fn header_of(jwt: &str) -> Value {
    let part = jwt.split('.').next().expect("a header");
    serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(part)
            .expect("base64url"),
    )
    .expect("JSON")
}

/// One issuer store per algorithm, opened once for a test's matrix. Opening a store runs
/// the passphrase KDF, so a store per pair made the matrix slow. Issuing only reads the
/// store and writes nothing to it, so every pair can share these.
fn issuers() -> Vec<(KeyAlgorithm, KeyStore)> {
    EVERY_ALGORITHM
        .into_iter()
        .map(|algorithm| {
            (
                algorithm,
                temp_store_with("sd-jwt-vc-alg", KEY_ID, algorithm),
            )
        })
        .collect()
}

/// One Holder key per curve, made once for a test's matrix.
fn holders() -> Vec<(KeyAlgorithm, KeyEntry)> {
    EVERY_ALGORITHM
        .into_iter()
        .map(|algorithm| (algorithm, holder_key(algorithm)))
        .collect()
}

/// A credential issued by `store` to `holder`, presented with Key Binding for
/// [`AUDIENCE`]. Returns the presentation.
fn presented(store: &KeyStore, holder: &KeyEntry) -> String {
    let credential = issue(
        store,
        KEY_ID,
        &payload(),
        &policy(),
        &Issuance {
            issuer: ISSUER,
            vct: &vct(),
            issued_at: ISSUED_AT,
            holder_key: Some(&holder.public_key()),
        },
    )
    .expect("issue");
    let reveal = digests(&credential, &["cathodeMaterial"]);
    let reveal: Vec<&str> = reveal.iter().map(String::as_str).collect();
    credential
        .present_with_key_binding(&reveal, &binding(), holder)
        .expect("the Holder binds")
        .serialise()
}

/// Every issuer algorithm, with a Holder key on every curve: the issuer signs under its
/// own algorithm, the credential names the Holder's key, and a presentation bound to it
/// verifies and reads to what was chosen.
#[test]
fn every_issuer_and_holder_algorithm_pair_issues_binds_and_verifies() {
    let issuers = issuers();
    let holders = holders();
    for (issuer, store) in &issuers {
        for (holder_alg, holder) in &holders {
            let label = format!(
                "issuer {} and holder {}",
                issuer.jose_alg(),
                holder_alg.jose_alg()
            );
            let presentation = presented(store, holder);

            let jwt = presentation
                .split('~')
                .next()
                .expect("the issuer-signed JWT");
            assert_eq!(
                header_of(jwt)["alg"],
                issuer.jose_alg(),
                "{label}: the issuer signs under its own algorithm"
            );
            let claims = verify(
                &presentation,
                &public_key(store),
                &vct(),
                now(),
                Some(&key_binding_policy()),
            )
            .unwrap_or_else(|e| panic!("{label}: a bound presentation must verify, got {e:?}"));

            assert_eq!(
                claims["cnf"],
                holder_key_confirmation(&holder.public_key()),
                "{label}: cnf names the Holder's key"
            );
            assert_eq!(
                claims[PASSPORT_CLAIM]["productGroupData"]["cathodeMaterial"],
                json!(["LiFePO4"]),
                "{label}: the claim the Holder revealed"
            );
            assert!(
                claims[PASSPORT_CLAIM]["productGroupData"]
                    .get("stateOfHealthPct")
                    .is_none(),
                "{label}: a claim the Holder did not reveal stays hidden"
            );
        }
    }
}

/// The Key Binding JWT is held to the audience the Verifier names. A presentation made
/// for another audience is refused, for every pair of algorithms, and the same
/// presentation verifies for its own.
#[test]
fn a_presentation_for_another_audience_is_refused_for_every_pair() {
    let issuers = issuers();
    let holders = holders();
    for (issuer, store) in &issuers {
        for (holder_alg, holder) in &holders {
            let label = format!(
                "issuer {} and holder {}",
                issuer.jose_alg(),
                holder_alg.jose_alg()
            );
            let presentation = presented(store, holder);
            let elsewhere = KeyBindingPolicy {
                audience: "https://elsewhere.example".to_owned(),
                ..key_binding_policy()
            };

            assert!(
                verify(
                    &presentation,
                    &public_key(store),
                    &vct(),
                    now(),
                    Some(&key_binding_policy())
                )
                .is_ok(),
                "{label}: control: the presentation verifies for its own audience"
            );
            assert_eq!(
                verify(
                    &presentation,
                    &public_key(store),
                    &vct(),
                    now(),
                    Some(&elsewhere)
                ),
                Err(SdJwtVcError::SdJwt(SdJwtError::KeyBindingRejected("aud"))),
                "{label}: a Key Binding JWT for another audience is refused"
            );
        }
    }
}

/// The JWK the issuer metadata publishes under `kid`.
fn published_jwk<'a>(metadata: &'a Value, kid: &Value) -> &'a Value {
    metadata["jwks"]["keys"]
        .as_array()
        .expect("a key set")
        .iter()
        .find(|key| &key["kid"] == kid)
        .unwrap_or_else(|| panic!("no published key under kid {kid}"))
}

/// An EC issuer key is published as an EC JWK under the `kid` its credentials name,
/// with `use: sig`. A verifier that looks the key up by that `kid` and reads it back
/// verifies the credential with it.
#[test]
fn an_ec_issuer_key_is_published_under_the_kid_its_credentials_name() {
    for (algorithm, kty, crv, alg) in EC_SHAPES {
        let store = temp_store_with("sd-jwt-vc-meta", KEY_ID, algorithm);
        let credential = issued(&store);
        let kid = header_of(credential.jwt())["kid"].clone();

        let metadata = build_issuer_metadata(&store, ISSUER, KEY_ID).expect("metadata");
        let jwk = published_jwk(&metadata, &kid);
        assert_eq!(jwk["kty"], kty, "{alg}: kty");
        assert_eq!(jwk["crv"], crv, "{alg}: crv");
        assert_eq!(jwk["alg"], alg, "{alg}: alg");
        assert_eq!(jwk["use"], "sig", "{alg}: use");

        let key = PublicKey::from_jwk(jwk).expect("the published key reads back");
        verify(&credential.serialise(), &key, &vct(), now(), None).unwrap_or_else(|e| {
            panic!(
                "{alg}: a verifier holding the published key must accept the credential, got {e:?}"
            )
        });
    }
}

/// A credential issued before a rotation from Ed25519 to an EC key, and one issued
/// after it, each verify against the metadata a verifier fetches after the rotation.
#[test]
fn a_rotation_to_an_ec_key_keeps_every_credential_verifiable() {
    for (algorithm, _, _, _) in EC_SHAPES {
        let name = algorithm.jose_alg();
        let store = temp_store(&format!("sd-jwt-vc-rotate-{name}"), KEY_ID);
        let before = issued(&store);

        store
            .rotate_key_to(KEY_ID, algorithm)
            .expect("rotate to an EC key");
        let after = issued(&store);
        let metadata = build_issuer_metadata(&store, ISSUER, KEY_ID).expect("metadata");

        for (when, credential) in [("before", &before), ("after", &after)] {
            let kid = header_of(credential.jwt())["kid"].clone();
            let key = PublicKey::from_jwk(published_jwk(&metadata, &kid))
                .expect("the published key reads back");
            verify(&credential.serialise(), &key, &vct(), now(), None).unwrap_or_else(|e| {
                panic!("{name}: a credential issued {when} the rotation must verify, got {e:?}")
            });
        }
    }
}
