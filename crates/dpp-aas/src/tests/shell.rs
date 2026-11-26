//! The shell and its core submodels.

use super::super::*;
use super::common::*;
use dpp_domain::Audience;
use dpp_domain::ProductGroup;

#[test]
fn build_aas_produces_five_core_submodels() {
    let passport = minimal_passport(ProductGroup::Electronics);
    let (shell, submodels) =
        build_aas_from_passport(&passport, identity_of(&passport), Audience::Public)
            .expect("masking");
    assert_eq!(submodels.len(), 5);
    let id_shorts: Vec<&str> = submodels.iter().map(|s| s.id_short.as_str()).collect();
    assert!(id_shorts.contains(&"ProductIdentification"));
    assert!(id_shorts.contains(&"ManufacturerInformation"));
    assert!(id_shorts.contains(&"EnvironmentalImpact"));
    assert!(id_shorts.contains(&"MaterialComposition"));
    assert!(id_shorts.contains(&"Repairability"));
    assert_eq!(shell.submodels.len(), 5);
}

#[test]
fn shell_submodel_refs_match_submodel_ids() {
    let passport = minimal_passport(ProductGroup::Battery);
    let (shell, submodels) =
        build_aas_from_passport(&passport, identity_of(&passport), Audience::Public)
            .expect("masking");
    let submodel_ids: Vec<&str> = submodels.iter().map(|s| s.id.as_str()).collect();
    for submodel_ref in &shell.submodels {
        // A shell's submodel list holds `ModelReference`s whose single key
        // names the target Submodel by id.
        assert_eq!(submodel_ref.ref_type, "ModelReference");
        let key = submodel_ref.keys.first().expect("a reference has one key");
        assert_eq!(key.key_type, "Submodel");
        assert!(
            submodel_ids.contains(&key.value.as_str()),
            "shell ref {} not found in submodels",
            key.value
        );
    }
}

#[test]
fn shell_has_correct_asset_information() {
    let passport = minimal_passport(ProductGroup::Textile);
    let (shell, _) = build_aas_from_passport(&passport, identity_of(&passport), Audience::Public)
        .expect("masking");
    // `minimal_passport` carries no product group data, so the identity is the
    // caller-chosen one — already a URI, so it is *not* wrapped.
    assert_eq!(
        shell.asset_information.global_asset_id,
        "urn:odal-node:test-asset:1"
    );
    let names: Vec<&str> = shell
        .asset_information
        .specific_asset_ids
        .iter()
        .map(|id| id.name.as_str())
        .collect();
    assert!(names.contains(&"assetKey"));

    // 🚨 `passportId`, not `serialId`. This slot carried `passport.id` — an
    // internal UUID, documented as an opaque link and explicitly not a legal
    // identifier — under a name an integrator reads as the unit's serial. The
    // envelope has a real `serial_number` field, which this fixture leaves
    // unset, so there is no serial to report and none is reported.
    assert!(names.contains(&"passportId"));
    assert!(
        !names.contains(&"serialId"),
        "a passport with no serial number must not claim one: {names:?}"
    );

    // `batchId` is `Restricted` in `PASSPORT_FIELD_DISCLOSURE`, so an anonymous
    // reader does not get one — and until the projection was wired through the
    // shared redaction, it did.
    //
    // The masking policy was resolved with `for_schema_version` alone, whose
    // `envelope_disclosure` carries only the universal conformity names. The
    // passport-level classes were never folded in, so every envelope field they
    // govern fell to the `Public` default here while the JSON view stripped it.
    // The same passport disclosed different things depending on which
    // representation you asked for.
    assert!(
        !names.contains(&"batchId"),
        "an anonymous reader must not receive the batch identifier: {names:?}"
    );
}
