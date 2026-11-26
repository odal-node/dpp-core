//! The environment: its shell, its submodels and its masking.

use super::super::*;
use super::common::*;
use dpp_domain::Audience;
use dpp_domain::{ProductGroup, ProductGroupData};

#[test]
fn environment_carries_the_shell_and_every_submodel() {
    let mut passport = minimal_passport(ProductGroup::Battery);
    passport.product_group_data = Some(ProductGroupData::Battery(Box::new(
        battery_data_with_due_diligence(),
    )));

    let env = build_aas_environment(&passport, identity_of(&passport), Audience::Public)
        .expect("environment is buildable");

    assert_eq!(env.asset_administration_shells.len(), 1);
    assert_eq!(env.submodels.len(), 6);
    assert!(
        env.concept_descriptions.is_empty(),
        "this crate coins no concept descriptions"
    );

    // The envelope must carry exactly what the pair form carries — the whole
    // point of one builder is that the two cannot disagree.
    let (shell, submodels) =
        build_aas_from_passport(&passport, identity_of(&passport), Audience::Public)
            .expect("pair form");
    assert_eq!(env.asset_administration_shells[0], shell);
    assert_eq!(env.submodels, submodels);
}

#[test]
fn environment_is_masked_for_its_audience() {
    let mut passport = minimal_passport(ProductGroup::Battery);
    passport.product_group_data = Some(ProductGroupData::Battery(Box::new(
        battery_data_with_due_diligence(),
    )));

    let public = build_aas_environment(&passport, identity_of(&passport), Audience::Public)
        .expect("public environment");
    let serialised = serde_json::to_string(&public).expect("serialises");

    // The envelope delegates to the masked builder, so a non-public field must
    // not appear at any depth — an envelope that assembled its own content
    // could reintroduce one. `cathodeMaterial` is Annex XIII point 2(a), which
    // Art. 77(2) withholds from the general public.
    assert!(
        !serialised.contains("cathodeMaterial"),
        "restricted field leaked into a public AAS Environment"
    );
    // …while a point 1(d) field must survive the same masking. Testing both
    // directions on one document is the point: a mask that drops everything
    // would pass the assertion above on its own.
    assert!(
        serialised.contains("dueDiligenceUrl"),
        "Annex XIII point 1(d) is public and must survive masking"
    );

    let restricted = build_aas_environment(
        &passport,
        identity_of(&passport),
        Audience::LegitimateInterest,
    )
    .expect("restricted environment");
    let restricted = serde_json::to_string(&restricted).expect("serialises");
    assert!(
        restricted.contains("cathodeMaterial"),
        "a legitimate-interest caller must receive point 2 content"
    );
    assert!(
        restricted.contains("dueDiligenceUrl"),
        "and must still receive the public tier"
    );
}

#[test]
fn environment_serialises_with_idta_field_names() {
    let passport = minimal_passport(ProductGroup::Battery);
    let env = build_aas_environment(&passport, identity_of(&passport), Audience::Public)
        .expect("buildable");
    let value = serde_json::to_value(&env).expect("serialises");

    for key in ["assetAdministrationShells", "submodels"] {
        assert!(
            value.get(key).is_some(),
            "Environment must carry '{key}' at the top level"
        );
    }

    // `conceptDescriptions` is absent rather than `[]`. The schema constrains
    // it to `minItems: 1`, so emitting the empty array — which this crate
    // always would, coining no concept descriptions — makes the whole document
    // invalid. Absent is both valid and the honest encoding of "none".
    assert!(
        value.get("conceptDescriptions").is_none(),
        "an empty conceptDescriptions array must not reach the wire"
    );
}
