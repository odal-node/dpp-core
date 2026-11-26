//! The asset an unsold-goods export names.

use super::super::*;
use super::common::*;

/// A disclosure is no trade item and has no product identifier, so the export names
/// the asset by a key the caller chose, and refuses a key that claims a name a
/// clause 5 scheme derives — the defect `AssetIdentity` exists to remove.
#[test]
fn an_unsold_goods_export_refuses_an_asset_name_a_scheme_derives() {
    for reserved in RESERVED_ASSET_ID_NAMES {
        let refused = build_unsold_goods_environment(
            &sample_unsold_goods_report(),
            reserved,
            "urn:odal-node:test-asset:1",
        )
        .expect_err("a reserved name is refused");
        assert!(
            matches!(refused, AasError::ReservedAssetIdName(_)),
            "{reserved}: {refused:?}"
        );
    }
}

/// An asset value with whitespace or a control character cannot appear in a URI, so the export refuses it.
#[test]
fn an_unsold_goods_export_refuses_an_asset_value_that_cannot_appear_in_a_uri() {
    let refused =
        build_unsold_goods_environment(&sample_unsold_goods_report(), "assetKey", "two words")
            .expect_err("whitespace cannot appear in a URI");
    assert!(
        matches!(refused, AasError::UnusableAssetIdentity(_)),
        "{refused:?}"
    );
}

/// The shell names the asset by the caller's key and references the one submodel
/// the export carries.
#[test]
fn an_unsold_goods_export_names_the_asset_and_references_its_submodel() {
    let environment = build_unsold_goods_environment(
        &sample_unsold_goods_report(),
        "assetKey",
        "urn:odal-node:test-asset:1",
    )
    .expect("a valid caller-chosen asset key");

    let [shell] = environment.asset_administration_shells.as_slice() else {
        panic!("one shell");
    };
    assert_eq!(
        shell.asset_information.global_asset_id,
        "urn:odal-node:test-asset:1"
    );
    assert_eq!(
        shell.asset_information.specific_asset_ids[0].name,
        "assetKey"
    );
    assert_eq!(shell.submodels.len(), 1);
    assert_eq!(environment.submodels.len(), 1);
    assert_eq!(
        shell.submodels[0].keys[0].value,
        environment.submodels[0].id
    );
}
