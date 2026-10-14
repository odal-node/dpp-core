//! The validity window — `exp`, `nbf`, and the claims that must never be
//! concealable.

use chrono::{TimeZone, Utc};
use serde_json::json;

use dpp_domain::ProductGroup;

use super::test_fixtures::*;
use super::{Issuance, SdJwtVcError, issue, vct_for};
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
    let credential = credential_with(&store, &[("exp", json!(expiry))]);

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
    let credential = credential_with(&store, &[("nbf", json!(start))]);

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

    // `iat` is in the list too: it is a root claim like the other two, and it can
    // only be malformed here because `issue` writes its own.
    for claim in ["nbf", "exp", "iat"] {
        let credential = credential_with(&store, &[(claim, json!("2026-09-16T00:00:00Z"))]);
        assert!(
            matches!(
                check(&store, &credential, now()),
                Err(SdJwtVcError::MalformedTemporalClaim(_))
            ),
            "{claim} as a string was accepted"
        );
    }
}

/// RFC 9901 clause 9.7 names the claims an issuer MUST NOT make selectively
/// disclosable. The ones `issue` sets are written straight into the root and only
/// the passport is walked for concealment, so none can be a disclosure, whatever
/// a policy or a schema says. Asserted over a bound credential, which has all four.
#[test]
fn the_claims_the_issuer_sets_are_never_disclosable() {
    let store = temp_store("sdjwtvc-critical", KEY_ID);
    let holder = temp_store("sdjwtvc-critical-holder", "holder")
        .load_key("holder")
        .expect("the key was just generated");
    let credential = issue(
        &store,
        KEY_ID,
        &payload(),
        &policy(),
        &Issuance {
            issuer: ISSUER,
            vct: &vct_for(ProductGroup::Battery, SCHEMA_VERSION),
            issued_at: ISSUED_AT,
            holder_key: Some(holder.verifying_key.to_bytes()),
        },
    )
    .expect("issue");

    assert!(
        !credential.disclosures().is_empty(),
        "control: the fixture conceals something, so there is a Disclosure to hide a claim in"
    );
    for claim in ["iss", "vct", "iat", "cnf"] {
        assert!(
            credential.digests_for_claim(claim).is_empty(),
            "{claim} was issued as a disclosure, so a holder can withhold it"
        );
    }
}

/// An `exp` in the clear is enforced on a presentation that reveals nothing. If it
/// were concealable, a holder could drop the disclosure and turn an expired
/// credential into an unbounded one — RFC 9901 clause 9.7 warns about exactly this.
#[test]
fn exp_is_still_enforced_on_a_presentation_that_reveals_nothing() {
    let store = temp_store("sdjwtvc-exp-cleartext", KEY_ID);
    let expiry = ISSUED_AT + 3600;
    let credential = credential_with(&store, &[("exp", json!(expiry))]);

    let parsed = dpp_crypto::sd_jwt::SdJwt::parse(&credential).unwrap();
    let bare = parsed
        .present(&[])
        .expect("nothing chosen is a valid choice")
        .serialise();
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
    let credential = credential_with(&store, &[("exp", json!(expiry as f64 + 0.5))]);

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
        let credential = credential_with(&store, &[("exp", value.clone())]);
        assert!(
            matches!(
                check(&store, &credential, now()),
                Err(SdJwtVcError::MalformedTemporalClaim(_))
            ),
            "{value} was accepted as an expiry"
        );
    }
}

/// `contains_key("iss")` accepted `null`, a number or an array, and the
/// credential then verified while naming no issuer — the one claim a relying
/// party needs in order to decide whose key should have signed it.
///
/// Minted here rather than through [`issue`], which writes its own `iss` and so
/// cannot produce the case. That is the point: the shape a verifier has to
/// refuse is one *somebody else's* issuer emitted, and a verifier that only
/// ever sees its own output is the one that grows this class of hole. Nothing is
/// concealed, so the verifier has to reach the claim check at all.
#[test]
fn an_iss_that_is_not_a_string_is_refused() {
    let store = temp_store("sdjwtvc-iss-shape", KEY_ID);

    for value in [json!(null), json!(42), json!(["a"]), json!({})] {
        let credential = credential_with(&store, &[("iss", value.clone())]);
        assert!(
            matches!(
                check(&store, &credential, now()),
                Err(SdJwtVcError::MissingClaim("iss"))
            ),
            "iss as {value} was accepted"
        );
    }
}

/// A passport member that shares a registered claim's name is an ordinary member,
/// whatever its depth: the policy classes it and, when it is non-public, it is
/// concealed. Registered names were once exempt from concealment at the payload
/// root, and at every depth before that, so a non-public `exp` travelled in
/// cleartext however the policy graded it. The passport now sits under its own
/// claim, the claims the issuer sets are not walked at all, and nothing is exempt.
///
/// Latent rather than live: no shipped schema classifies an `exp`, `aud` or `cnf`
/// today. This test supplies the policy that makes it so, which is the only way to
/// pin the rule without waiting for a schema to add such a field.
#[test]
fn a_passport_member_sharing_a_registered_claim_name_is_concealed_at_every_depth() {
    use std::collections::HashMap;

    use dpp_domain::Disclosure;
    use dpp_domain::access::ProductGroupAccessPolicy;

    let store = temp_store("sdjwtvc-nested-exp", KEY_ID);

    let mut field_disclosure = HashMap::new();
    field_disclosure.insert("exp".to_owned(), Disclosure::Individual);
    let mut envelope_disclosure = HashMap::new();
    envelope_disclosure.insert("exp".to_owned(), Disclosure::Individual);
    let policy = ProductGroupAccessPolicy {
        name: "nested-registered-name".into(),
        product_group: "battery".into(),
        field_disclosure,
        envelope_disclosure,
        default_disclosure: Disclosure::Public,
        public_envelope_fields: Default::default(),
    };

    let payload = json!({
        "id": "018f3a4c-0000-7000-8000-000000000001",
        "productGroup": "battery",
        "schemaVersion": SCHEMA_VERSION,
        "exp": "an envelope member that is not an expiry",
        "productGroupData": { "exp": "a non-public value that is not an expiry" },
    });

    let credential = super::issue(
        &store,
        KEY_ID,
        &payload,
        &policy,
        &Issuance {
            issuer: ISSUER,
            vct: &vct_for(ProductGroup::Battery, SCHEMA_VERSION),
            issued_at: ISSUED_AT,
            holder_key: None,
        },
    )
    .expect("issue")
    .serialise();

    let parsed = dpp_crypto::sd_jwt::SdJwt::parse(&credential).unwrap();
    assert_eq!(
        parsed.digests_for_claim("exp").len(),
        2,
        "a non-public `exp` travelled in cleartext, at the passport's root or below it"
    );

    // And the token's own claims are untouched: `iat` stays in cleartext, or a
    // verifier could not read it before disclosure.
    assert!(
        parsed.digests_for_claim("iat").is_empty(),
        "the root `iat` was concealed"
    );
}
