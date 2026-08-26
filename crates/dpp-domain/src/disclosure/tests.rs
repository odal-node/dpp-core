//! The disclosure lattice: which audience reaches which Annex XIII points.

use super::*;
use crate::credential::{PassportCredential, PassportCredentialSubject};
use serde_json::Value;

#[test]
fn authorities_do_not_see_individual_item_data() {
    // Art. 77(2)(b) assigns notified bodies, market surveillance and the
    // Commission Annex XIII points 2 and 3 — not point 4. This is the case
    // an ordinal tier cannot express, and the reason the model changed.
    assert!(!Audience::Authority.may_see(Disclosure::Individual));
    assert!(Audience::LegitimateInterest.may_see(Disclosure::Individual));
}

#[test]
fn legitimate_interest_does_not_see_conformity_evidence() {
    // Point 3 (test reports) is authority-only under Art. 77(2)(b).
    assert!(!Audience::LegitimateInterest.may_see(Disclosure::Conformity));
    assert!(Audience::Authority.may_see(Disclosure::Conformity));
}

/// The key names the *data*, not the asker. This is the property that lets a
/// stored signature survive ESPR naming a different actor taxonomy, so it is
/// asserted literally rather than round-tripped.
#[test]
fn a_disclosure_key_names_classes_never_an_audience() {
    assert_eq!(Audience::Public.disclosure_key(), "public");
    assert_eq!(
        Audience::LegitimateInterest.disclosure_key(),
        "public+restricted+individual"
    );
    assert_eq!(
        Audience::Authority.disclosure_key(),
        "public+restricted+conformity"
    );

    for audience in [
        Audience::Public,
        Audience::LegitimateInterest,
        Audience::Authority,
    ] {
        let key = audience.disclosure_key();
        for name in ["legitimate", "authority", "public_", "audience"] {
            assert!(
                !key.contains(name) || key == "public",
                "{key} leaks an audience name into a durable artefact key"
            );
        }
    }
}

/// Key construction is order-independent in its input and fixed in its
/// output: two nodes handed the same set in different orders must produce
/// byte-identical keys, or the same view signs under two names.
#[test]
fn a_disclosure_key_is_canonical_regardless_of_input_order() {
    let forward = disclosure_key(&[
        Disclosure::Public,
        Disclosure::Restricted,
        Disclosure::Individual,
    ]);
    let reversed = disclosure_key(&[
        Disclosure::Individual,
        Disclosure::Restricted,
        Disclosure::Public,
    ]);
    assert_eq!(forward, reversed);
    assert_eq!(forward, "public+restricted+individual");
    assert_eq!(disclosure_key(&[]), "");
}

/// The set an audience is keyed by must be exactly what `may_see` grants —
/// if these drift, a view is signed under a key that overstates or
/// understates what it contains.
#[test]
fn the_disclosure_set_agrees_with_may_see() {
    for audience in [
        Audience::Public,
        Audience::LegitimateInterest,
        Audience::Authority,
    ] {
        let set = audience.disclosure_set();
        for class in [
            Disclosure::Public,
            Disclosure::Restricted,
            Disclosure::Conformity,
            Disclosure::Individual,
        ] {
            assert_eq!(
                set.contains(&class),
                audience.may_see(class),
                "{audience:?} / {class:?}: disclosure_set disagrees with may_see"
            );
        }
    }
}

#[test]
fn neither_non_public_audience_contains_the_other() {
    // The defining property of a lattice: if either audience were a superset
    // of the other, an ordinal ranking would suffice and this type would be
    // unnecessary. Each sees something the other does not.
    let all = [
        Disclosure::Public,
        Disclosure::Restricted,
        Disclosure::Conformity,
        Disclosure::Individual,
    ];
    let authority_only = all
        .iter()
        .any(|d| Audience::Authority.may_see(*d) && !Audience::LegitimateInterest.may_see(*d));
    let interest_only = all
        .iter()
        .any(|d| Audience::LegitimateInterest.may_see(*d) && !Audience::Authority.may_see(*d));
    assert!(authority_only && interest_only);
}

#[test]
fn point_two_is_shared_and_public_is_universal() {
    for audience in [
        Audience::Public,
        Audience::LegitimateInterest,
        Audience::Authority,
    ] {
        assert!(audience.may_see(Disclosure::Public));
    }
    // Annex XIII point 2 goes to both non-public audiences.
    assert!(Audience::LegitimateInterest.may_see(Disclosure::Restricted));
    assert!(Audience::Authority.may_see(Disclosure::Restricted));
    assert!(!Audience::Public.may_see(Disclosure::Restricted));
}

#[test]
fn public_sees_nothing_restricted() {
    for class in [
        Disclosure::Restricted,
        Disclosure::Conformity,
        Disclosure::Individual,
    ] {
        assert!(!Audience::Public.may_see(class));
    }
}

#[test]
fn new_passport_credential_guarantees_vc_base_context_and_type() {
    let vc = PassportCredential::new(
        "did:web:issuer.example.com".into(),
        PassportCredentialSubject {
            id: "urn:uuid:00000000-0000-0000-0000-000000000000".into(),
            payload_hash: "deadbeef".into(),
        },
    );
    // VCDM v2 requires the base context to be the first @context entry.
    assert_eq!(
        vc.context.first().and_then(Value::as_str),
        Some(PassportCredential::VC_BASE_CONTEXT)
    );
    assert!(
        vc.credential_type
            .contains(&"VerifiableCredential".to_string())
    );
    assert!(vc.id.starts_with("urn:uuid:"));
}
