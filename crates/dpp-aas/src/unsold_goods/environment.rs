//! [`build_unsold_goods_environment`] — a disclosure as an AAS Environment.

use dpp_domain::unsold_goods::UnsoldGoodsReport;

use super::submodel::build_unsold_goods_submodel;
use crate::builder::AasError;
use crate::identity::AssetIdentity;
use crate::model::{AasEnvironment, AasSemId, AasShell, AssetInformation, SpecificAssetId};

/// Project an unsold-goods disclosure as an AAS Environment, for a file export or
/// an AASX package assembled for a reporting authority.
///
/// # The asset is the caller's to name
///
/// An AAS shell's `globalAssetId` is built from what the asset *is*, and a
/// disclosure is no trade item: it covers a financial year across many products and
/// has no product identifier by law (Impl. Reg. (EU) 2026/2 Annex I identifies an
/// undertaking and a period). Inventing a `globalAssetId` that looks like one would
/// put a fabricated trade-item identifier into a document an integrator's toolchain
/// treats as authoritative. So the caller says what the asset is, as an
/// [`AssetIdentity::Named`] — a key of its own choosing, validated exactly as any
/// other caller-chosen key is, and refused if it claims a name a clause 5 scheme
/// derives.
///
/// # No masking
///
/// There is no audience here. A disclosure is published in full on the operator's
/// own website (ESPR Art. 24(1)), so there is nothing in it to withhold from any
/// reader, and it is not a passport, so it is not filtered through the passport
/// disclosure policy.
///
/// # Errors
///
/// [`AasError::ReservedAssetIdName`] if `asset_name` is one a scheme derives or is
/// empty, and [`AasError::UnusableAssetIdentity`] if `asset_value` cannot appear in
/// a URI.
pub fn build_unsold_goods_environment(
    report: &UnsoldGoodsReport,
    asset_name: &str,
    asset_value: &str,
) -> Result<AasEnvironment, AasError> {
    let identity = AssetIdentity::Named {
        name: asset_name,
        value: asset_value,
    };
    identity.validate()?;

    let submodel = build_unsold_goods_submodel(
        report,
        &format!("urn:odal-node:unsold-goods:{asset_value}:report"),
    );
    let shell = AasShell {
        id: format!("urn:odal-node:aas:unsold-goods:{asset_value}"),
        id_short: "UnsoldGoodsDisclosure".into(),
        model_type: "AssetAdministrationShell".into(),
        asset_information: AssetInformation {
            asset_kind: "Instance".into(),
            global_asset_id: identity.global_asset_id(),
            specific_asset_ids: vec![SpecificAssetId {
                name: identity.asset_id_name()?.to_owned(),
                value: identity.value().to_owned(),
            }],
        },
        submodels: vec![AasSemId::submodel(&submodel.id)],
    };

    Ok(AasEnvironment {
        asset_administration_shells: vec![shell],
        submodels: vec![submodel],
        concept_descriptions: Vec::new(),
    })
}
