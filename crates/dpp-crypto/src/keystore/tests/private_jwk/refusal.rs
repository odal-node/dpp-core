//! The refusals of [`KeyEntry::from_private_jwk`]: one test for each rule the constructor
//! enforces. Each refused JWK is a mutated copy of one of the W3C VC-JOSE-COSE suite's key
//! files, or a key generated while the test runs. The helpers and constants are those of
//! `private_jwk_tests`. No private key is written into this file.

use base64::Engine;
use serde_json::json;

use super::super::super::KeyEntry;
use super::super::super::entry::SigningMaterial;
use super::super::super::error::PrivateJwkError;
use super::super::private_jwk::keys::{
    B64, ORDERS, SUITE, d_octets, public, refused, secret, with_d,
};
use crate::jws::algorithm::KeyAlgorithm;

/// A public-only JWK has no `d`, so it is refused as not private.
#[test]
fn a_public_only_jwk_is_refused_as_not_private() {
    for (name, _) in SUITE {
        assert_eq!(
            refused(&public(name)),
            PrivateJwkError::NotPrivate,
            "{name}"
        );
    }
}

/// A `kty` and `crv` pair that names no supported key is refused, including RSA and
/// secp256k1, and a curve paired with the wrong key type.
#[test]
fn an_unsupported_key_type_or_curve_is_refused() {
    for (kty, crv) in [
        ("RSA", "P-256"),
        ("EC", "secp256k1"),
        ("OKP", "P-256"),
        ("EC", "Ed25519"),
        ("OKP", "X25519"),
    ] {
        let mut jwk = secret("p256");
        jwk["kty"] = json!(kty);
        jwk["crv"] = json!(crv);
        assert_eq!(
            refused(&jwk),
            PrivateJwkError::UnsupportedKey,
            "{kty}/{crv}"
        );
    }
}

/// An `alg` that names another key's algorithm is refused: ES384 on a P-256 key, and
/// so on for each curve.
#[test]
fn an_alg_naming_another_key_is_refused() {
    for (name, other) in [
        ("p256", "ES384"),
        ("p384", "ES512"),
        ("p521", "ES256"),
        ("ed25519", "ES256"),
    ] {
        let mut jwk = secret(name);
        jwk["alg"] = json!(other);
        assert_eq!(
            refused(&jwk),
            PrivateJwkError::AlgorithmMismatch,
            "{name} as {other}"
        );
    }
}

/// `RS256` is not an algorithm of any key this crate builds.
#[test]
fn an_rs256_alg_is_refused() {
    for (name, _) in SUITE {
        let mut jwk = secret(name);
        jwk["alg"] = json!("RS256");
        assert_eq!(refused(&jwk), PrivateJwkError::AlgorithmMismatch, "{name}");
    }
}

/// `alg: none` is refused, as it is everywhere in this crate.
#[test]
fn an_alg_none_is_refused() {
    for (name, _) in SUITE {
        let mut jwk = secret(name);
        jwk["alg"] = json!("none");
        assert_eq!(refused(&jwk), PrivateJwkError::AlgorithmMismatch, "{name}");
    }
}

/// An `alg` that is not a string is refused, whatever its value.
#[test]
fn a_non_string_alg_is_refused() {
    for (name, _) in SUITE {
        for value in [json!(256), json!(null), json!(["ES256"])] {
            let mut jwk = secret(name);
            jwk["alg"] = value.clone();
            assert_eq!(
                refused(&jwk),
                PrivateJwkError::AlgorithmMismatch,
                "{name} as {value}"
            );
        }
    }
}

/// `d` taken from one key file, with the public point of another key of the same
/// algorithm, derives a public key other than the JWK's.
#[test]
fn a_private_member_with_another_keys_public_point_is_refused() {
    for (name, algorithm) in SUITE {
        let other = SigningMaterial::generate(algorithm).public_key().jwk();
        let mut jwk = secret(name);
        for member in ["x", "y"] {
            if let Some(value) = other.get(member) {
                jwk[member] = value.clone();
            }
        }
        assert_eq!(refused(&jwk), PrivateJwkError::KeyPairMismatch, "{name}");
    }
}

/// `d` one octet longer than the field size is refused as malformed.
#[test]
fn a_d_one_octet_too_long_is_refused() {
    for (name, _) in SUITE {
        let mut octets = d_octets(&secret(name));
        octets.push(0);
        let jwk = with_d(secret(name), &octets);
        assert_eq!(refused(&jwk), PrivateJwkError::PrivateKeyInvalid, "{name}");
    }
}

/// `d` one octet shorter than the field size, on a published key, is refused as
/// malformed. The derived key would differ too, so this shows the outcome; the length
/// check is what the leading-zero test below isolates.
#[test]
fn a_d_one_octet_too_short_is_refused() {
    for (name, _) in SUITE {
        let mut octets = d_octets(&secret(name));
        octets.pop();
        let jwk = with_d(secret(name), &octets);
        assert_eq!(refused(&jwk), PrivateJwkError::PrivateKeyInvalid, "{name}");
    }
}

/// A P-256 scalar with a leading zero octet, written without that octet, is 31 octets.
/// Upstream `from_slice` pads it back to the same scalar, so the derived key is the
/// JWK's own and only the exact-length check refuses it.
#[test]
fn a_short_p256_scalar_is_refused_though_upstream_would_pad_it() {
    let generated = (0..10_000)
        .map(|_| SigningMaterial::generate(KeyAlgorithm::EcdsaP256))
        .find(|key| key.secret()[0] == 0)
        .expect("about one P-256 scalar in 256 has a leading zero octet");
    let scalar = generated.secret();

    let full = with_d(generated.public_key().jwk(), &scalar);
    assert_eq!(
        KeyEntry::from_private_jwk(&full)
            .expect("the full-length scalar builds")
            .public_key(),
        generated.public_key()
    );

    assert!(
        p256::ecdsa::SigningKey::from_slice(&scalar[1..]).is_ok(),
        "upstream pads a short scalar, so it cannot be what refuses this one"
    );
    let short = with_d(generated.public_key().jwk(), &scalar[1..]);
    assert_eq!(refused(&short), PrivateJwkError::PrivateKeyInvalid);
}

/// A `d` that is not base64url is refused as malformed.
#[test]
fn a_d_that_is_not_base64url_is_refused() {
    for (name, _) in SUITE {
        let mut jwk = secret(name);
        jwk["d"] = json!("not*base64url");
        assert_eq!(refused(&jwk), PrivateJwkError::PrivateKeyInvalid, "{name}");
    }
}

/// A `d` a megabyte long is refused as malformed on each suite key. Its encoded length
/// is wrong, so it is refused before anything is decoded.
#[test]
fn an_oversized_d_is_refused_on_each_suite_key() {
    for (name, _) in SUITE {
        let mut jwk = secret(name);
        jwk["d"] = json!("A".repeat(1 << 20));
        assert_eq!(refused(&jwk), PrivateJwkError::PrivateKeyInvalid, "{name}");
    }
}

/// A `d` of zero octets is not a private scalar on any EC curve.
#[test]
fn a_zero_d_is_refused_on_each_ec_curve() {
    for (name, _, _) in ORDERS {
        let zero = vec![0u8; d_octets(&secret(name)).len()];
        let jwk = with_d(secret(name), &zero);
        assert_eq!(refused(&jwk), PrivateJwkError::PrivateKeyInvalid, "{name}");
    }
}

/// A `d` equal to the group order `n` is not a private scalar on any EC curve. The
/// constants are checked first: `n − 1` is a scalar and `n` is refused.
#[test]
fn a_d_equal_to_the_group_order_is_refused_on_each_ec_curve() {
    for (name, algorithm, order_hex) in ORDERS {
        let order = hex::decode(order_hex).expect("hex");
        let mut below = order.clone();
        *below.last_mut().expect("a non-empty order") -= 1;
        assert!(
            SigningMaterial::from_secret(algorithm, &below).is_ok(),
            "{name}: n − 1 is a scalar"
        );
        assert!(
            SigningMaterial::from_secret(algorithm, &order).is_err(),
            "{name}: n is not"
        );

        let jwk = with_d(secret(name), &order);
        assert_eq!(refused(&jwk), PrivateJwkError::PrivateKeyInvalid, "{name}");
    }
}

/// An `x` one octet short is refused as a public key: RFC 7518 §6.2.1.2 and
/// RFC 8037 §2 require the full length of each coordinate.
#[test]
fn a_truncated_public_coordinate_is_refused() {
    for (name, _) in SUITE {
        let mut jwk = secret(name);
        let mut x = B64
            .decode(jwk["x"].as_str().expect("x"))
            .expect("base64url");
        x.pop();
        jwk["x"] = json!(B64.encode(&x));
        assert_eq!(refused(&jwk), PrivateJwkError::PublicKeyInvalid, "{name}");
    }
}

/// A `y` that is not on the curve is refused as a public key. Flipping the last bit of
/// a point's `y` leaves the curve, so the point is refused before `d` is read.
#[test]
fn an_off_curve_public_point_is_refused() {
    for (name, _, _) in ORDERS {
        let mut jwk = secret(name);
        let mut y = B64
            .decode(jwk["y"].as_str().expect("y"))
            .expect("base64url");
        let last = y.len() - 1;
        y[last] ^= 0x01;
        jwk["y"] = json!(B64.encode(&y));
        assert_eq!(refused(&jwk), PrivateJwkError::PublicKeyInvalid, "{name}");
    }
}
