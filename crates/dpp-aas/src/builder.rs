//! [`build_aas_from_passport`] — the primary entry point mapping a passport to
//! a complete AAS shell + submodels.

use dpp_domain::access::redact_passport;
use dpp_domain::catalog::Granularity;
use dpp_domain::{Audience, Passport};

use super::identity::AssetIdentity;
use super::model::{
    AasEnvironment, AasSemId, AasShell, AasSubmodel, AssetInformation, SpecificAssetId,
};
use super::product_groups;

/// Why an AAS projection could not be built.
#[derive(Debug)]
#[non_exhaustive]
pub enum AasError {
    /// The passport did not survive a masking round-trip. Structural, not a
    /// permissions failure: the disclosure policy removed a field the passport
    /// requires to exist, so no honest projection can be produced for that
    /// audience. Fails closed rather than emitting a partial shell.
    Masking(String),
    /// A caller-chosen asset identity claimed a `specificAssetId` name that a
    /// clause 5 scheme derives, which would let the escape hatch file one kind
    /// of value under another kind's label — the defect `AssetIdentity` exists
    /// to remove. See `RESERVED_ASSET_ID_NAMES`.
    ReservedAssetIdName(String),
    /// A caller-chosen asset identity carries a value that cannot appear in a
    /// URI, so no honest `globalAssetId` can be built from it.
    ///
    /// Checked here because nothing downstream does: the AAS reference
    /// implementation verifies `globalAssetId`'s length and not its syntax.
    UnusableAssetIdentity(String),
}

impl std::fmt::Display for AasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Masking(m) => write!(f, "passport did not survive masking: {m}"),
            Self::ReservedAssetIdName(n) => write!(
                f,
                "asset id name '{n}' is reserved for an identifier scheme and may \
                 not be chosen by a caller"
            ),
            Self::UnusableAssetIdentity(m) => {
                write!(f, "asset identity cannot form an identifier: {m}")
            }
        }
    }
}

impl std::error::Error for AasError {}

/// Map a [`Passport`] and a caller-chosen asset identity into a complete AAS
/// shell + submodels, carrying only what `audience` may see.
///
/// Returns `(AasShell, Vec<AasSubmodel>)`. The shell's `submodels` list
/// contains only ID references; the payloads are in the `Vec`.
///
/// `gtin` becomes the `globalAssetId` and a `specificAssetId` entry for GS1
/// Digital Link routing. It is a parameter, and not read off the passport, so
/// that the asset identity stays the caller's to decide — the unsold-goods
/// mapper's module note records the case that forced it.
///
/// 🚨 The `specificAssetId` it populates is named `gtin` unconditionally, so a
/// caller passing anything else — an EN 18219 scheme 2 or 3 identifier, an
/// internal asset key — gets that value under a GS1 label. Pre-existing, and
/// wider than the passport's own identifier: it is the shell's asset identity,
/// not `ProductGroupData::product_identifier`, which the product-group submodel
/// emits separately as `productIdentifier`.
///
/// # Masking
///
/// The passport is filtered **before** any mapper sees it, through the same
/// [`redact_passport`] seam the public view uses — not filtered afterwards,
/// and never by the mappers themselves. This is the whole contract: a mapper
/// that assembled its own field list would eventually disagree with the
/// canonical one, and the direction it disagrees in is the direction that
/// leaks. There is deliberately no unmasked entry point.
///
/// The round-trip works because every non-public field in the catalog is
/// optional on its typed struct, so a redacted document still deserialises with
/// those fields absent and the mappers simply do not emit them.
///
/// A product group the catalog does not describe has **no** field classified, so the
/// filter alone would pass its whole payload through. Such a product group is reduced
/// to its `product_group` discriminant instead, before any mapper runs, for **every**
/// audience — an unmodelled product group has no field policy for any of them, so a
/// credentialed reader must not receive more of it than an anonymous one. The
/// product group stays identified: `ProductIdentification` carries the tag.
///
/// That is a property of this function, not of any particular caller: handing it
/// a complete, unredacted passport is safe.
///
/// # Errors
///
/// [`AasError::Masking`] if the filtered document no longer deserialises into a
/// `Passport` — a required field was classified non-public, which is a policy
/// defect rather than a caller error.
pub fn build_aas_from_passport(
    passport: &Passport,
    identity: AssetIdentity<'_>,
    audience: Audience,
) -> Result<(AasShell, Vec<AasSubmodel>), AasError> {
    // Before anything is built from it: a name no scheme derives, and a value
    // that can appear in a URI. Fails closed rather than emitting a shell whose
    // asset identifier is not one.
    identity.validate()?;

    let passport = &mask(passport, audience)?;
    let passport_id = passport.id.to_string();

    let mut specific_asset_ids = vec![
        SpecificAssetId {
            name: identity.asset_id_name()?.to_owned(),
            value: identity.value().to_owned(),
        },
        // 🚨 Named for what it is. This slot used to be `serialId` and carried
        // `passport.id` — an internal UUID, documented elsewhere as an opaque
        // link and explicitly *not* a legal identifier, published in a document
        // an integrator reads as the unit's serial number. `serial_number` is a
        // real field on the envelope and was never read here at all.
        SpecificAssetId {
            name: "passportId".into(),
            value: passport_id.clone(),
        },
    ];
    if let Some(serial) = &passport.serial_number {
        specific_asset_ids.push(SpecificAssetId {
            name: "serialId".into(),
            value: serial.clone(),
        });
    }
    if let Some(batch) = &passport.batch_id {
        specific_asset_ids.push(SpecificAssetId {
            name: "batchId".into(),
            value: batch.clone(),
        });
    }

    let mut submodels = vec![
        product_groups::build_product_identification_submodel(passport),
        product_groups::build_manufacturer_submodel(passport),
        product_groups::build_environmental_impact_submodel(passport),
        product_groups::build_material_composition_submodel(passport),
        product_groups::build_repairability_submodel(passport),
    ];
    if let Some(sd) = &passport.product_group_data {
        submodels.push(product_groups::build_product_group_submodel(
            sd,
            &passport_id,
        ));
    }

    let shell = AasShell {
        id: format!("urn:odal-node:aas:{passport_id}"),
        id_short: "DigitalProductPassport".into(),
        model_type: "AssetAdministrationShell".into(),
        asset_information: AssetInformation {
            asset_kind: asset_kind_for(passport.granularity).into(),
            global_asset_id: identity.global_asset_id(),
            specific_asset_ids,
        },
        submodels: submodels
            .iter()
            .map(|s| AasSemId::submodel(&s.id))
            .collect(),
    };

    Ok((shell, submodels))
}

/// Apply the one audience redaction to the whole passport document, before any
/// mapper sees it.
fn mask(passport: &Passport, audience: Audience) -> Result<Passport, AasError> {
    // One redaction, shared with every other surface that serves a passport.
    //
    // This used to resolve the policy and apply its own backstops here. That was
    // not wrong — it was version-pinned, it fail-closed on an unknown product
    // group, and it carried an *extra* guard the other surfaces did not. That
    // asymmetry was the problem: the strictest reading of "who sees what" lived
    // on whichever surface someone had most recently thought about, and a rule
    // held in three places had already drifted into three answers.
    //
    // `redact_passport` now owns policy resolution, the filter, both backstops
    // and the proof strip. Everything downstream of this line is projection.
    let view = redact_passport(passport, audience).into_value();

    serde_json::from_value(view)
        .map_err(|e| AasError::Masking(format!("redacted document no longer valid: {e}")))
}

/// Build a complete [`AasEnvironment`] — the self-contained document form,
/// shells and submodels in one payload.
///
/// Delegates to [`build_aas_from_passport`], so the passport is masked for
/// `audience` before any mapper sees it and there is no envelope-shaped route
/// around the disclosure seam. Every consumer that needs a whole-document AAS —
/// an HTTP door serving `application/aas+json`, an AASX package, a conformance
/// check — builds it here, so the encodings cannot disagree about content.
///
/// # Errors
///
/// Propagates [`AasError::Masking`] unchanged.
pub fn build_aas_environment(
    passport: &Passport,
    identity: AssetIdentity<'_>,
    audience: Audience,
) -> Result<AasEnvironment, AasError> {
    let (shell, submodels) = build_aas_from_passport(passport, identity, audience)?;
    Ok(AasEnvironment {
        asset_administration_shells: vec![shell],
        submodels,
        concept_descriptions: Vec::new(),
    })
}

/// The AAS `assetKind` a passport's registration level implies.
///
/// 🚨 `"Instance"` was hardcoded, under a doc comment asserting a passport
/// "describes one manufactured item or batch, never a product type
/// definition". [`Granularity::Model`] is exactly a product type definition —
/// *one passport covering every unit sharing a model's specifications* — so
/// the claim was false for one of the three levels the model admits.
///
/// `Batch` and `Item` both describe manufactured things and stay `"Instance"`.
/// `None` also stays `"Instance"`: granularity is `Option` because the envelope
/// is additive-only and no adopted act has fixed a level, so absence means "not
/// stated" and the prior behaviour is the honest default for it.
const fn asset_kind_for(granularity: Option<Granularity>) -> &'static str {
    match granularity {
        Some(g) if g.describes_a_type() => "Type",
        _ => "Instance",
    }
}
