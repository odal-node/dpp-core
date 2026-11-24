//! Shared fixture builders for the integration tests in `tests/` (and the
//! `dpp-benches` crate, which depends on this library as a dev-dependency).
//!
//! These are the product group-agnostic envelope / actor shapes that were hand-rolled
//! at multiple call sites before this module existed.

use chrono::Utc;
use dpp_domain::{
    CarbonFootprint, ManufacturerInfo, MaterialEntry, OperatorRole, Passport, PassportId,
    PassportStatus, ProductGroup, ProductGroupData, RepairabilityScore, ResponsibleOperator,
};
use dpp_vc::{CredentialRole, DppCredentialSubject};

/// A base passport with the product group-agnostic fields populated so the five core
/// AAS submodels (identification, manufacturer, environmental, materials,
/// repairability) all exercise their optional branches. Callers override
/// individual fields via struct-update syntax for scenario-specific values.
pub fn base_passport(
    product_group: ProductGroup,
    product_group_data: ProductGroupData,
    schema_version: &str,
) -> Passport {
    // A caller-supplied version is honoured only if it is one this build knows;
    // otherwise the product group's current version is used.
    //
    // Disclosure classes are now sourced from the declared schema version, so a
    // fixture that declares an older version while carrying data built from the
    // *current* typed structs classifies none of the newer fields — and the
    // masking backstop strips them, leaving a submodel with nothing in it. That
    // is the fixture being wrong rather than the code, and it was invisible
    // while the catalog's disclosure map ignored versions entirely.
    let catalog = dpp_domain::ProductGroupCatalog::new();
    let schema_version = catalog.get(product_group.catalog_key()).map_or_else(
        || schema_version.to_owned(),
        |d| {
            if d.schema_versions.iter().any(|v| v == schema_version) {
                d.current_schema_version.clone()
            } else {
                schema_version.to_owned()
            }
        },
    );
    let schema_version = schema_version.as_str();
    let now = Utc::now();
    Passport {
        id: PassportId::new(),
        batch_id: Some("LOT-X-0001".into()),
        serial_number: None,
        product_name: format!("{} reference product", product_group.catalog_key()),
        applicable_instruments: dpp_domain::InstrumentCatalog::new()
            .instrument_refs_for(product_group.catalog_key()),
        granularity: None,
        product_group,
        manufacturer: ManufacturerInfo {
            name: "Acme Manufacturing GmbH".into(),
            address: "Hauptstraße 1, 10115 Berlin, DE".into(),
            country: Some("DE".into()),
            did_web_url: Some("https://acme.example.com/.well-known/did.json".into()),
            registered_trade_name: None,
            electronic_address: None,
        },
        materials: vec![MaterialEntry {
            name: "Primary material".into(),
            weight_kg: 1.5,
            recycled_pct: Some(20.0),
            country_of_origin: Some("DE".into()),
        }],
        co2e_per_unit: Some(CarbonFootprint::from_kg(12.0)),
        repairability_score: Some(RepairabilityScore::from_scalar(6.0)),
        compliance_result: None,
        lint_result: None,
        product_group_data: Some(product_group_data),
        status: PassportStatus::Draft,
        qr_code_url: None,
        carrier_serial: None,
        jws_signature: None,
        public_jws_signature: None,
        disclosure_signatures: Default::default(),
        personal_data: Default::default(),
        created_at: now,
        updated_at: now,
        published_at: None,
        placed_on_market_date: None,
        schema_version: schema_version.into(),
        retention_locked: false,
        version: 1,
        supersedes_id: None,
        derived_from: Vec::new(),
        component_refs: Vec::new(),
        life_status: None,
        retention_until: None,
        product_id: None,
        commodity_code: None,
        operator_identifier: None,
        responsible_operator: None,
        facility: None,
        seal: None,
    }
}

/// A [`ResponsibleOperator`] for transfer-of-responsibility / provenance tests.
pub fn make_operator(
    did: &str,
    name: &str,
    role: OperatorRole,
    country: &str,
) -> ResponsibleOperator {
    ResponsibleOperator {
        did: did.into(),
        name: name.into(),
        role,
        eu_operator_id: None,
        eu_operator_id_scheme: None,
        identifier: None,
        country: country.into(),
        registered_trade_name: None,
        postal_address: Some("Hauptstraße 1, 10115 Berlin".into()),
        electronic_address: Some("compliance@acme.example.com".into()),
    }
}

/// A [`DppCredentialSubject`] for audience / verifiable-credential tests.
pub fn make_subject(
    did: &str,
    name: &str,
    role: CredentialRole,
    product_groups: Vec<String>,
) -> DppCredentialSubject {
    DppCredentialSubject {
        id: did.into(),
        name: name.into(),
        role,
        country: "DE".into(),
        product_groups,
        product_categories: vec![],
    }
}

/// A minimal, well-formed unsold-goods disclosure in the Annex I shape of
/// Commission Implementing Regulation (EU) 2026/2.
///
/// One line, a treatment split totalling 100, and a CN heading outside Annex II
/// so the depth lint stays quiet.
#[must_use]
pub fn unsold_goods_report() -> dpp_domain::UnsoldGoodsReport {
    use chrono::NaiveDate;
    use dpp_domain::{
        CnCategory, DiscardReason, DiscardedProductLine, DiscardedQuantity, DisclosingEntity,
        DisclosureScope, FinancialYear, LegalEntityIdentifier, UnsoldGoodsReport,
        WasteTreatmentSplit,
    };

    UnsoldGoodsReport {
        entity: DisclosingEntity {
            name: "Example Retail Group SA".into(),
            identifier: LegalEntityIdentifier::Euid {
                value: "LUB123456789".into(),
            },
            scope: DisclosureScope::Standalone,
        },
        financial_year: FinancialYear {
            start: NaiveDate::from_ymd_opt(2027, 1, 1).expect("valid date"),
            end: NaiveDate::from_ymd_opt(2027, 12, 31).expect("valid date"),
        },
        lines: vec![DiscardedProductLine {
            cn_categories: vec![CnCategory::parse("6203").expect("valid CN heading")],
            description: "Men's suits, ensembles, jackets and trousers".into(),
            units_discarded: DiscardedQuantity::measured(1_200),
            weight_kg: DiscardedQuantity::estimated(430),
            packaging_included: false,
            reason: Some(DiscardReason::DamagedOrContaminated),
            reason_detail: None,
            treatment: WasteTreatmentSplit {
                preparing_for_reuse_pct: 20,
                recycling_pct: 50,
                other_recovery_pct: 20,
                disposal_pct: 5,
                unknown_pct: 5,
            },
        }],
        measures_taken: "Introduced pre-season demand forecasting across all lines.".into(),
        measures_planned: "Extending the donation offer window to twelve weeks.".into(),
    }
}

/// The schema-compat fixture of a product group at a schema version: a payload that
/// version's schema accepts, as the wire carries it.
///
/// # Panics
///
/// If the group has no fixture at that version, which is a gap in the fixtures.
pub fn schema_compat_fixture(key: &str, version: &str) -> serde_json::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../dpp-domain/tests/fixtures/schema-compat")
        .join(key)
        .join(format!("v{version}.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("no schema-compat fixture for '{key}' at {version}: {e}"));
    serde_json::from_str(&raw).expect("a fixture is JSON")
}

/// A passport's wire form with its product group data replaced by `data`.
///
/// The envelope comes from the real serialiser, and only `productGroupData` is swapped,
/// as JSON, so the credential carries exactly the keys `data` holds.
///
/// # Panics
///
/// If `data` is not an object.
pub fn passport_with(
    group: &ProductGroup,
    version: &str,
    data: serde_json::Value,
) -> serde_json::Value {
    let placeholder: ProductGroupData = serde_json::from_value(serde_json::json!({
        "productGroup": "packaging",
        "materialFamily": "placeholder",
    }))
    .expect("an unmodelled product group deserialises");
    let mut passport = serde_json::to_value(base_passport(group.clone(), placeholder, version))
        .expect("a passport serialises");

    let tag = serde_json::to_value(group).expect("a product group serialises");
    let mut data = data;
    data.as_object_mut()
        .expect("product group data is an object")
        .insert("productGroup".to_owned(), tag.clone());
    let wire = passport.as_object_mut().expect("a passport is an object");
    wire.insert("productGroup".to_owned(), tag);
    wire.insert("productGroupData".to_owned(), data);
    passport
}

/// The published schema of a product group at a version, as JSON.
///
/// # Panics
///
/// If the group has no schema at that version.
pub fn schema_for(key: &str, version: &str) -> serde_json::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../dpp-domain/schemas")
        .join(key)
        .join(format!("v{version}.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("no schema for '{key}' at {version}: {e}"));
    serde_json::from_str(&raw).expect("a schema is JSON")
}

/// `node` with a `$ref` into the schema's own definitions followed.
fn resolved<'a>(
    schema: &'a serde_json::Value,
    node: &'a serde_json::Value,
) -> &'a serde_json::Value {
    node.get("$ref")
        .and_then(serde_json::Value::as_str)
        .and_then(|reference| reference.strip_prefix('#'))
        .and_then(|pointer| schema.pointer(pointer))
        .unwrap_or(node)
}

/// For each step of `parents`, from the root of the data, whether the schema types that
/// member as an array. A step the schema does not describe is taken as an object.
fn parent_kinds<'a>(schema: &serde_json::Value, parents: &[&'a str]) -> Vec<(&'a str, bool)> {
    let mut node = Some(resolved(schema, schema));
    parents
        .iter()
        .map(|name| {
            node = node.and_then(|parent| {
                let parent = resolved(schema, parent);
                // The elements of an array sit at the array's own path.
                let parent = parent.get("items").map_or(parent, |i| resolved(schema, i));
                parent
                    .get("properties")
                    .and_then(|properties| properties.get(*name))
                    .map(|member| resolved(schema, member))
            });
            let array = node.is_some_and(|member| match member.get("type") {
                Some(serde_json::Value::String(kind)) => kind == "array",
                Some(serde_json::Value::Array(kinds)) => kinds.iter().any(|k| k == "array"),
                _ => member.get("items").is_some(),
            });
            (*name, array)
        })
        .collect()
}

/// `data` with the members `policy` classes above `Public`, each where its key puts it:
/// `usageHistory.recordedAt` is `recordedAt` of the object at `usageHistory`, made if
/// the fixture has none, and of every object of an array that holds it. A parent the
/// fixture lacks is made as the schema types it, so a member of an array's elements sits
/// in an array of one object, as a passport carries it.
///
/// With `carry` every such member is there. Without it the objects that would hold them
/// are there with one member of no class instead, so the two shapes differ in nothing
/// but what is hidden. Credentials issued from both show whether an issuer's output
/// depends on how many restricted members a passport happens to carry.
///
/// A member already in `data` is kept as it is.
pub fn with_restricted_members(
    mut data: serde_json::Value,
    schema: &serde_json::Value,
    policy: &dpp_domain::access::ProductGroupAccessPolicy,
    carry: bool,
) -> serde_json::Value {
    let mut keys: Vec<&String> = policy
        .field_disclosure
        .iter()
        .filter(|(_, class)| **class != dpp_domain::Disclosure::Public)
        .map(|(key, _)| key)
        .collect();
    keys.sort();
    for key in keys {
        let mut segments: Vec<&str> = key.split('.').collect();
        let leaf = segments.pop().expect("a key has a leaf");
        let parents = parent_kinds(schema, &segments);
        place_member(&mut data, &parents, leaf, carry);
    }
    data
}

fn place_member(node: &mut serde_json::Value, parents: &[(&str, bool)], leaf: &str, carry: bool) {
    use serde_json::{Value, json};
    match node {
        Value::Array(items) => {
            for item in items {
                place_member(item, parents, leaf, carry);
            }
        }
        Value::Object(map) => match parents.split_first() {
            Some(((first, array), rest)) => {
                let child = map
                    .entry((*first).to_owned())
                    .or_insert_with(|| if *array { json!([{}]) } else { json!({}) });
                place_member(child, rest, leaf, carry);
            }
            None if carry => {
                map.entry(leaf.to_owned())
                    .or_insert_with(|| json!("restricted"));
            }
            None => {
                map.entry("publicNote".to_owned())
                    .or_insert_with(|| json!("public"));
            }
        },
        _ => {}
    }
}

/// `passport`'s envelope with every field `policy` classes above `Public` there, or with
/// none of them, to match [`with_restricted_members`] for the envelope. A field the
/// product group opens to the public is left as it is.
///
/// # Panics
///
/// If `passport` is not an object.
pub fn with_restricted_envelope_members(
    passport: &mut serde_json::Value,
    policy: &dpp_domain::access::ProductGroupAccessPolicy,
    carry: bool,
) {
    let envelope = passport.as_object_mut().expect("a passport is an object");
    for (name, class) in &policy.envelope_disclosure {
        let opened = policy.public_envelope_fields.contains(name);
        if *class == dpp_domain::Disclosure::Public || name.contains('.') || opened {
            continue;
        }
        if carry {
            envelope
                .entry(name.clone())
                .or_insert_with(|| serde_json::json!("restricted"));
        } else {
            envelope.remove(name);
        }
    }
}
