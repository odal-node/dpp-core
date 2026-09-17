//! The validity window — `exp`, `nbf`, and the claims that must never be
//! concealable.

use chrono::{TimeZone, Utc};
use serde_json::json;

use dpp_domain::ProductGroup;

use super::test_fixtures::*;
use super::{SdJwtVcError, vct_for, verify};
use crate::test_support::temp_store;

/// Both claims are OPTIONAL in the profile, so their absence means "no bound"
/// and must not be read as "expired".
#[test]
fn a_credential_without_exp_or_nbf_is_unbounded() {
    let store = temp_store("sdjwtvc-unbounded", KEY_ID);
    let credential = issued(&store).serialise();

    assert!(check(&store, &credential, Utc.timestamp_opt(0, 0).unwrap()).is_ok());
    assert!(
        check(
            &store,
            &credential,
            Utc.timestamp_opt(4_000_000_000, 0).unwrap()
        )
        .is_ok()
    );
}

#[test]
fn an_expired_credential_is_refused() {
    let store = temp_store("sdjwtvc-expired", KEY_ID);
    let expiry = ISSUED_AT + 3600;
    let credential = issued_with(&store, &[("exp", json!(expiry))]);

    // A second before expiry it verifies; at and after it, it does not.
    assert!(
        check(
            &store,
            &credential,
            Utc.timestamp_opt(expiry - 1, 0).unwrap()
        )
        .is_ok()
    );
    assert!(matches!(
        check(&store, &credential, Utc.timestamp_opt(expiry, 0).unwrap()),
        Err(SdJwtVcError::Expired { .. })
    ));
    assert!(matches!(
        check(
            &store,
            &credential,
            Utc.timestamp_opt(expiry + 86_400, 0).unwrap()
        ),
        Err(SdJwtVcError::Expired { .. })
    ));
}

#[test]
fn a_credential_presented_before_nbf_is_refused() {
    let store = temp_store("sdjwtvc-nbf", KEY_ID);
    let start = ISSUED_AT + 3600;
    let credential = issued_with(&store, &[("nbf", json!(start))]);

    assert!(matches!(
        check(
            &store,
            &credential,
            Utc.timestamp_opt(start - 1, 0).unwrap()
        ),
        Err(SdJwtVcError::NotYetValid { .. })
    ));
    assert!(check(&store, &credential, Utc.timestamp_opt(start, 0).unwrap()).is_ok());
}

/// A malformed temporal claim must not degrade to "absent", which is the
/// unbounded case — the permissive reading would be reachable by sending
/// rubbish.
#[test]
fn a_temporal_claim_that_is_not_a_numericdate_is_refused() {
    let store = temp_store("sdjwtvc-malformed", KEY_ID);

    for claim in ["nbf", "exp"] {
        let credential = issued_with(&store, &[(claim, json!("2026-09-16T00:00:00Z"))]);
        assert!(
            matches!(
                check(&store, &credential, now()),
                Err(SdJwtVcError::MalformedTemporalClaim(_))
            ),
            "{claim} as a string was accepted"
        );
    }

    // `iat` is not in that list because it cannot get there: `issue` writes its
    // own, and the payload walk cannot displace it. Asserted rather than
    // assumed, since it is the reason the loop above is short.
    let credential = issued_with(&store, &[("iat", json!("not-a-date"))]);
    let disclosed = verify(
        &credential,
        &public_key_b64(&store),
        &vct_for(ProductGroup::Battery, SCHEMA_VERSION),
        now(),
    )
    .expect("issuer-set iat is well-formed");
    assert_eq!(disclosed["iat"], json!(ISSUED_AT));
}

/// RFC 9901 clause 9.7 names the claims an issuer MUST NOT make selectively
/// disclosable. None of them may appear as a disclosure, whatever the policy
/// says — and the policy is asked directly here so the test does not pass
/// merely because no schema happens to mention them.
#[test]
fn validity_controlling_claims_are_never_disclosable() {
    let store = temp_store("sdjwtvc-critical", KEY_ID);
    let credential = issued_with(
        &store,
        &[
            ("exp", json!(ISSUED_AT + 3600)),
            ("nbf", json!(ISSUED_AT - 10)),
            ("aud", json!("https://verifier.example")),
            ("cnf", json!({"jwk": {"kty": "OKP"}})),
        ],
    );
    let parsed = dpp_crypto::sd_jwt::SdJwt::parse(&credential).unwrap();

    for claim in ["iss", "aud", "exp", "nbf", "cnf", "vct", "iat"] {
        assert!(
            parsed.digests_for_claim(claim).is_empty(),
            "{claim} was issued as a disclosure, so a holder can withhold it"
        );
    }
}

/// `exp` is issued in cleartext, not as a disclosure. If it were concealable, a
/// holder could drop the disclosure and turn an expired credential into an
/// unbounded one — RFC 9901 clause 9.7 warns about exactly this.
#[test]
fn exp_is_not_selectively_disclosable() {
    let store = temp_store("sdjwtvc-exp-cleartext", KEY_ID);
    let expiry = ISSUED_AT + 3600;
    let credential = issued_with(&store, &[("exp", json!(expiry))]);

    let parsed = dpp_crypto::sd_jwt::SdJwt::parse(&credential).unwrap();
    assert!(
        parsed.digests_for_claim("exp").is_empty(),
        "exp travels as a disclosure, so a holder can withhold its own expiry"
    );

    // And it is therefore still enforced on a presentation that reveals nothing.
    let bare = parsed.present(&[]).serialise();
    assert!(matches!(
        check(&store, &bare, Utc.timestamp_opt(expiry + 1, 0).unwrap()),
        Err(SdJwtVcError::Expired { .. })
    ));
}

/// RFC 7519 clause 2 permits a non-integer `NumericDate`, and `as_i64` refused
/// one — so a conformant credential carrying `exp: 1789003600.5` was rejected as
/// malformed. Refusing a valid credential is the defect; the fraction is now
/// kept rather than truncated, which is what makes the second half of this test
/// distinguish "parsed" from "parsed and rounded down".
#[test]
fn a_fractional_numericdate_is_accepted_and_keeps_its_fraction() {
    let store = temp_store("sdjwtvc-fractional", KEY_ID);
    let expiry = ISSUED_AT + 3600;
    let credential = issued_with(&store, &[("exp", json!(expiry as f64 + 0.5))]);

    // Half a second before the fractional expiry: still valid. Truncating the
    // fraction would place expiry at `expiry` exactly, and `now >= instant`
    // would already have refused this.
    check(&store, &credential, Utc.timestamp_opt(expiry, 0).unwrap())
        .expect("a credential is valid up to its fractional expiry");

    // And one second after it, expired — so the claim is enforced, not ignored.
    assert!(matches!(
        check(
            &store,
            &credential,
            Utc.timestamp_opt(expiry + 1, 0).unwrap()
        ),
        Err(SdJwtVcError::Expired { .. })
    ));
}

/// The fail-closed direction has to survive the fractional parse: anything that
/// is not a finite number still reads as malformed, because a temporal claim
/// that silently fails to parse would read as an *absent* one, and absent means
/// unbounded.
#[test]
fn a_non_finite_or_out_of_range_numericdate_is_still_refused() {
    let store = temp_store("sdjwtvc-notfinite", KEY_ID);

    for value in [json!(1.0e300), json!(-1.0e300)] {
        let credential = issued_with(&store, &[("exp", value.clone())]);
        assert!(
            matches!(
                check(&store, &credential, now()),
                Err(SdJwtVcError::MalformedTemporalClaim(_))
            ),
            "{value} was accepted as an expiry"
        );
    }
}

/// Mint a credential the way a foreign issuer would: sign a payload we choose,
/// with the right `typ`, using the fixture's key so the signature verifies.
///
/// Nothing is concealed, so there are no disclosures and no `_sd` — the point is
/// the shape of `iss`, and a verifier must reach the claim check at all.
fn foreign_credential_with_iss(
    store: &dpp_crypto::keystore::KeyStore,
    iss: serde_json::Value,
) -> String {
    let jwt_payload = json!({
        "iss": iss,
        "vct": vct_for(ProductGroup::Battery, SCHEMA_VERSION),
        "iat": ISSUED_AT,
    });
    let jwt = dpp_crypto::jws::sign_typed(store, KEY_ID, &jwt_payload, Some(super::TYP))
        .expect("fixture key signs");
    dpp_crypto::sd_jwt::SdJwt::new(jwt, Vec::new()).serialise()
}

/// `contains_key("iss")` accepted `null`, a number or an array, and the
/// credential then verified while naming no issuer — the one claim a relying
/// party needs in order to decide whose key should have signed it.
///
/// Minted here rather than through [`issue`], which writes its own `iss` and so
/// cannot produce the case. That is the point: the shape a verifier has to
/// refuse is one *somebody else's* issuer emitted, and a verifier that only
/// ever sees its own output is the one that grows this class of hole.
#[test]
fn an_iss_that_is_not_a_string_is_refused() {
    let store = temp_store("sdjwtvc-iss-shape", KEY_ID);

    for value in [json!(null), json!(42), json!(["a"]), json!({})] {
        let credential = foreign_credential_with_iss(&store, value.clone());
        assert!(
            matches!(
                check(&store, &credential, now()),
                Err(SdJwtVcError::MissingClaim("iss"))
            ),
            "iss as {value} was accepted"
        );
    }
}

/// `NEVER_CONCEALED` exempts eight registered JWT claims, and it was applied at
/// **every** recursion depth — so any nested field that happened to share one of
/// those names was skipped before it was ever classified, and travelled in
/// cleartext however the policy graded it.
///
/// Latent rather than live: no shipped schema classifies a nested `exp`, `aud`
/// or `cnf` today, and `issue`'s own note says these "survive by accident". This
/// test supplies the policy that ends the accident, which is the only way to
/// pin the rule without waiting for a schema to add such a field.
#[test]
fn a_nested_field_sharing_a_registered_claim_name_is_still_concealed() {
    use std::collections::HashMap;

    use dpp_domain::Disclosure;
    use dpp_domain::access::ProductGroupAccessPolicy;

    let store = temp_store("sdjwtvc-nested-exp", KEY_ID);

    let mut field_disclosure = HashMap::new();
    field_disclosure.insert("exp".to_owned(), Disclosure::Individual);
    let policy = ProductGroupAccessPolicy {
        name: "nested-registered-name".into(),
        product_group: "battery".into(),
        field_disclosure,
        envelope_disclosure: HashMap::new(),
        default_disclosure: Disclosure::Public,
    };

    let payload = json!({
        "id": "018f3a4c-0000-7000-8000-000000000001",
        "productGroup": "battery",
        "schemaVersion": SCHEMA_VERSION,
        "productGroupData": { "exp": "a non-public value that is not an expiry" },
    });

    let credential = super::issue(
        &store,
        KEY_ID,
        &payload,
        &policy,
        ISSUER,
        &vct_for(ProductGroup::Battery, SCHEMA_VERSION),
        ISSUED_AT,
    )
    .expect("issue")
    .serialise();

    let parsed = dpp_crypto::sd_jwt::SdJwt::parse(&credential).unwrap();
    assert!(
        !parsed.digests_for_claim("exp").is_empty(),
        "a nested, non-public `exp` travelled in cleartext — the root-claim \
         exemption is leaking into the payload"
    );

    // And the root exemption it protects is untouched: the token's own `iat`
    // stays in cleartext, or a verifier could not read it before disclosure.
    assert!(
        parsed.digests_for_claim("iat").is_empty(),
        "the root `iat` was concealed — the exemption is now too narrow"
    );
}
