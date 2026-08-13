//! [`build_aas_from_passport`] — the primary entry point mapping a passport to
//! a complete AAS shell + submodels.

use dpp_domain::access::{SectorAccessPolicy, filter_by_audience};
use dpp_domain::{Audience, Passport};

use super::model::{
    AasEnvironment, AasSemId, AasShell, AasSubmodel, AssetInformation, SpecificAssetId,
};
use super::sectors;

/// Why an AAS projection could not be built.
#[derive(Debug)]
pub enum AasError {
    /// The passport did not survive a masking round-trip. Structural, not a
    /// permissions failure: the disclosure policy removed a field the passport
    /// requires to exist, so no honest projection can be produced for that
    /// audience. Fails closed rather than emitting a partial shell.
    Masking(String),
}

impl std::fmt::Display for AasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Masking(m) => write!(f, "passport did not survive masking: {m}"),
        }
    }
}

impl std::error::Error for AasError {}

/// Map a [`Passport`] and its GS1 GTIN into a complete AAS shell + submodels,
/// carrying only what `audience` may see.
///
/// Returns `(AasShell, Vec<AasSubmodel>)`. The shell's `submodels` list
/// contains only ID references; the payloads are in the `Vec`.
///
/// `gtin` is the 14-digit GTIN identifying the product model. It becomes the
/// `globalAssetId` and a `specificAssetId` entry for GS1 Digital Link routing.
///
/// # Masking
///
/// The passport is filtered **before** any mapper sees it, through the same
/// [`filter_by_audience`] seam the public view uses — not filtered afterwards,
/// and never by the mappers themselves. This is the whole contract: a mapper
/// that assembled its own field list would eventually disagree with the
/// canonical one, and the direction it disagrees in is the direction that
/// leaks. There is deliberately no unmasked entry point.
///
/// The round-trip works because every non-public field in the catalog is
/// optional on its typed struct, so a redacted document still deserialises with
/// those fields absent and the mappers simply do not emit them.
///
/// A sector the catalog does not describe has **no** field classified, so the
/// filter alone would pass its whole payload through. Such a sector is reduced
/// to its `sector` discriminant instead, before any mapper runs, for **every**
/// audience — an unmodelled sector has no field policy for any of them, so a
/// credentialed reader must not receive more of it than an anonymous one. The
/// sector stays identified: `ProductIdentification` carries the tag.
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
    gtin: &str,
    audience: Audience,
) -> Result<(AasShell, Vec<AasSubmodel>), AasError> {
    let passport = &mask(passport, audience)?;
    let passport_id = passport.id.to_string();

    let mut specific_asset_ids = vec![
        SpecificAssetId {
            name: "gtin".into(),
            value: gtin.to_owned(),
        },
        SpecificAssetId {
            name: "serialId".into(),
            value: passport_id.clone(),
        },
    ];
    if let Some(batch) = &passport.batch_id {
        specific_asset_ids.push(SpecificAssetId {
            name: "batchId".into(),
            value: batch.clone(),
        });
    }

    let mut submodels = vec![
        sectors::build_product_identification_submodel(passport),
        sectors::build_manufacturer_submodel(passport),
        sectors::build_environmental_impact_submodel(passport),
        sectors::build_material_composition_submodel(passport),
        sectors::build_repairability_submodel(passport),
    ];
    if let Some(sd) = &passport.sector_data {
        submodels.push(sectors::build_sector_submodel(sd, &passport_id));
    }

    let shell = AasShell {
        id: format!("urn:odal-node:aas:{passport_id}"),
        id_short: "DigitalProductPassport".into(),
        model_type: "AssetAdministrationShell".into(),
        asset_information: AssetInformation {
            asset_kind: "Instance".into(),
            global_asset_id: format!("urn:odal-node:product:{gtin}"),
            specific_asset_ids,
        },
        submodels: submodels
            .iter()
            .map(|s| AasSemId::submodel(&s.id))
            .collect(),
    };

    Ok((shell, submodels))
}

/// Apply the sector's disclosure policy to the whole passport document.
///
/// Policy resolution mirrors the public view: the sector's own classes from the
/// catalog when it has an entry, otherwise the passport-level defaults — so a
/// sector this build has never seen is masked by the conservative default
/// rather than passed through unfiltered.
fn mask(passport: &Passport, audience: Audience) -> Result<Passport, AasError> {
    // Sourced from the passport's *own* schema version, not the catalog's
    // current map: the projection must be filtered by the disclosure classes
    // that were in force when this passport was signed, or a reclassification
    // would silently change what an already-published passport projects.
    //
    // `None` covers both an unknown sector and an unparseable or unregistered
    // version, so it still doubles as the unknown-sector test below — and an
    // unrecognised version now fails closed instead of borrowing whatever the
    // current map happens to say.
    let sector_policy = SectorAccessPolicy::for_schema_version(
        passport.sector.catalog_key(),
        &passport.schema_version,
    );
    let sector_is_unknown = sector_policy.is_none();
    let policy = sector_policy.unwrap_or_else(SectorAccessPolicy::passport_default);

    let document =
        serde_json::to_value(passport).map_err(|e| AasError::Masking(format!("serialise: {e}")))?;
    let mut filtered = filter_by_audience(&document, &policy, audience).filtered_data;

    if sector_is_unknown {
        redact_unknown_sector_data(&mut filtered);
    } else {
        redact_unclassified_sector_fields(&mut filtered, &policy);
    }

    serde_json::from_value(filtered)
        .map_err(|e| AasError::Masking(format!("redacted document no longer valid: {e}")))
}

/// Drop any `sectorData` key the passport's **declared schema version** does not
/// declare.
///
/// # Why this exists
///
/// Disclosure classes are sourced from the schema version a passport was
/// validated against, which is what keeps a published passport filtered by the
/// rules that produced its signatures. The corollary is the hazard: a key that
/// version does not declare is classified by nobody, and an unclassified key
/// falls to the policy default — `Public`.
///
/// So the version-sourced policy, which is safer than the catalog map in every
/// other respect, is *less* safe in exactly one: it under-applies where the map
/// over-applied. Under-applying disclosure is a leak.
///
/// Raising the default to `Restricted` does not work, because this policy is
/// applied to the whole document and the passport envelope's public fields
/// (`productName`, `status`, …) are not declared in any sector schema — they
/// would all vanish. The guard therefore has to be structural and scoped to
/// `sectorData`, which is the same shape as
/// [`redact_unknown_sector_data`] one level down: where nobody has classified
/// the content, keep only what is accounted for.
///
/// Reaching this needs an *invalid* passport — every sector schema sets
/// `additionalProperties: false`, so validation already rejects a document
/// carrying keys its version does not declare. This is defence in depth, and it
/// runs for **every** audience: an unclassified field is not more disclosable to
/// a credentialed reader than to an anonymous one.
fn redact_unclassified_sector_fields(
    document: &mut serde_json::Value,
    policy: &SectorAccessPolicy,
) {
    let Some(sector_data) = document
        .as_object_mut()
        .and_then(|o| o.get_mut("sectorData"))
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };
    // `sector` is the variant tag, not a schema property, and the document must
    // keep round-tripping through `SectorData`.
    // `field_disclosure` was built from this schema version's own properties,
    // so its keys are exactly what the version accounts for. Reusing it avoids a
    // second registry lookup that could disagree with the policy in force.
    sector_data.retain(|key, _| key == "sector" || policy.field_disclosure.contains_key(key));
}

/// Reduce an unrecognised sector's `sectorData` to its `sector` tag alone.
///
/// The filter above cannot help here. Both policies it can choose from carry
/// `default_disclosure: Public`, which is safe only because a *listed* sector's
/// non-public fields are listed — and an unlisted sector has nothing listed, so
/// every field it carries is unlisted, and every unlisted field is Public. The
/// filter therefore passes the whole payload through, and the more unusual the
/// sector, the more certain it is that nobody has classified its fields.
///
/// So the backstop is structural rather than policy-driven: with no field
/// policy, keep only the discriminant. Nothing is lost by dropping the tag from
/// the sector submodel itself — `ProductIdentification` carries `sector`
/// independently — but it is kept here so the redacted document still
/// round-trips through [`SectorData::Other`](dpp_domain::SectorData::Other),
/// which reads its variant from that key.
///
/// **Applied for every audience, not just `Public`.** An unrecognised sector has
/// no field policy for *any* audience, so a credentialed reader must not receive
/// more of it than an anonymous one. This mirrors, deliberately and for the same
/// stated reason, the backstop the platform applies when it renders a public
/// view — the two must not disagree about what an unmodelled sector discloses.
/// Drop any `sectorData` key the passport's **declared schema version** does not
/// declare.
///
/// # Why this exists
///
/// Disclosure classes are sourced from the schema version a passport was
/// validated against, which is what keeps a published passport filtered by the
/// rules that produced its signatures. The corollary is the hazard: a key that
/// version does not declare is classified by nobody, and an unclassified key
/// falls to the policy default — `Public`.
///
/// So the version-sourced policy, which is safer than the catalog map in every
/// other respect, is *less* safe in exactly one: it under-applies where the map
/// over-applied. Under-applying disclosure is a leak.
///
/// Raising the default to `Restricted` does not work, because this policy is
/// applied to the whole document and the passport envelope's public fields
/// (`productName`, `status`, …) are not declared in any sector schema — they
/// would all vanish. The guard therefore has to be structural and scoped to
/// `sectorData`, which is the same shape as
/// [`redact_unknown_sector_data`] one level down: where nobody has classified
/// the content, keep only what is accounted for.
///
/// Reaching this needs an *invalid* passport — every sector schema sets
/// `additionalProperties: false`, so validation already rejects a document
/// carrying keys its version does not declare. This is defence in depth, and it
/// runs for **every** audience: an unclassified field is not more disclosable to
/// a credentialed reader than to an anonymous one.
fn redact_unclassified_sector_fields(
    document: &mut serde_json::Value,
    policy: &SectorAccessPolicy,
) {
    let Some(sector_data) = document
        .as_object_mut()
        .and_then(|o| o.get_mut("sectorData"))
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };
    // `sector` is the variant tag, not a schema property, and the document must
    // keep round-tripping through `SectorData`.
    // `field_disclosure` was built from this schema version's own properties,
    // so its keys are exactly what the version accounts for. Reusing it avoids a
    // second registry lookup that could disagree with the policy in force.
    sector_data.retain(|key, _| key == "sector" || policy.field_disclosure.contains_key(key));
}

fn redact_unknown_sector_data(document: &mut serde_json::Value) {
    let Some(object) = document.as_object_mut() else {
        return;
    };
    let Some(sector_data) = object.get("sectorData") else {
        return;
    };
    // An untagged payload is a legacy record rather than an unknown sector, and
    // is left to the filter — matching how the platform draws the same line.
    let Some(tag) = sector_data
        .get("sector")
        .and_then(serde_json::Value::as_str)
        .filter(|tag| !tag.is_empty())
        .map(str::to_owned)
    else {
        return;
    };
    object.insert("sectorData".into(), serde_json::json!({ "sector": tag }));
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
    gtin: &str,
    audience: Audience,
) -> Result<AasEnvironment, AasError> {
    let (shell, submodels) = build_aas_from_passport(passport, gtin, audience)?;
    Ok(AasEnvironment {
        asset_administration_shells: vec![shell],
        submodels,
        concept_descriptions: Vec::new(),
    })
}
