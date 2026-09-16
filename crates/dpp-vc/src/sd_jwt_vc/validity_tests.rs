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
