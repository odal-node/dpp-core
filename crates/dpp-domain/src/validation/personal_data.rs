//! The personal-data check: a marked field with a value needs a statement, and a
//! statement needs a marked field.
//!
//! [`crate::personal_data`] states the rule and the law behind it. This is where
//! it is enforced, at write time, before anything is signed: the prohibition in
//! ESPR Art. 10(1)(e) is on *storing*, and a stored draft is already storage.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use crate::field_error::FieldError;
use crate::instrument::{CustomerPersonalData, InstrumentCatalog};
use crate::passport::Passport;
use crate::personal_data::{LawfulBasis, PersonalDataStatement};

use super::functions::{default_catalog, default_registry, product_group_data_instance};

/// The schema keyword that marks a property as able to hold personal data.
pub const PERSONAL_DATA_MARK: &str = "x-personal-data";

/// The embedded instrument catalog, built once.
fn default_instruments() -> &'static InstrumentCatalog {
    static INSTRUMENTS: OnceLock<InstrumentCatalog> = OnceLock::new();
    INSTRUMENTS.get_or_init(InstrumentCatalog::new)
}

/// The fields a product group's schema marks as able to hold personal data, at
/// one schema version, as dotted paths inside `productGroupData` —
/// `usageHistory.operatingConditions.note`. `None` when the embedded registry
/// holds no such schema.
///
/// Paths are written the way disclosure paths are: only an object property adds
/// a segment, so an array element sits where its array does, and a local `$ref`
/// is followed and walked at the path of whatever referred to it.
///
/// A mark under `additionalProperties`, `patternProperties` or a conditional
/// keyword has no fixed path, so it is not collected. A test over every shipped
/// schema fails if any mark is left uncollected, so that is a refusal to author
/// one there rather than a silent gap.
#[must_use]
pub fn personal_data_marks(product_group_key: &str, version: &str) -> Option<BTreeSet<String>> {
    let version: semver::Version = version.parse().ok()?;
    let json = default_registry().get(product_group_key, &version)?;
    let root: serde_json::Value = serde_json::from_str(json).ok()?;
    let mut marks = BTreeSet::new();
    collect_marks(&root, "", &root, &mut Vec::new(), &mut marks);
    Some(marks)
}

fn collect_marks(
    node: &serde_json::Value,
    path: &str,
    root: &serde_json::Value,
    active_refs: &mut Vec<String>,
    out: &mut BTreeSet<String>,
) {
    let Some(object) = node.as_object() else {
        return;
    };

    if let Some(pointer) = object.get("$ref").and_then(serde_json::Value::as_str)
        && let Some(target) = pointer.strip_prefix('#').and_then(|p| root.pointer(p))
        && !active_refs.iter().any(|seen| seen == pointer)
    {
        active_refs.push(pointer.to_owned());
        collect_marks(target, path, root, active_refs, out);
        active_refs.pop();
    }

    if let Some(properties) = object.get("properties").and_then(|p| p.as_object()) {
        for (name, property) in properties {
            let child_path = if path.is_empty() {
                name.clone()
            } else {
                format!("{path}.{name}")
            };
            if property.get(PERSONAL_DATA_MARK) == Some(&serde_json::Value::Bool(true)) {
                out.insert(child_path.clone());
            }
            collect_marks(property, &child_path, root, active_refs, out);
        }
    }

    if let Some(items) = object.get("items") {
        collect_marks(items, path, root, active_refs, out);
    }

    for key in ["allOf", "anyOf", "oneOf"] {
        if let Some(branches) = object.get(key).and_then(|b| b.as_array()) {
            for branch in branches {
                collect_marks(branch, path, root, active_refs, out);
            }
        }
    }
}

/// Check a passport's personal-data statements against its product group's
/// marks.
///
/// Refuses, naming every problem at once:
///
/// - a marked field that has a value and no statement in `personalData`;
/// - a statement about a field the schema does not mark;
/// - an `outside` statement whose record identifier is empty;
/// - an `outside` statement on a basis that a governing act does not admit.
///
/// It never reads what a marked field says. Whether text is personal data is
/// not something a pattern can decide, and the statement is the operator's
/// answer to that question.
///
/// # Which schema, and which acts
///
/// Marks are read from the schema at the product group's **current** version,
/// which is the version [`super::validate_product_group_data`] checks the data
/// against. This is a write-time check, and a write is made at the current
/// version.
///
/// The governing acts are the passport's
/// [`applicable_instruments`](Passport::applicable_instruments). When it records
/// none, the acts the instrument catalog binds to the product group stand in
/// for it, so that a passport which has not recorded its acts yet is not
/// excused from their conditions. Each act's condition is resolved through
/// [`InstrumentCatalog::customer_personal_data_for`], so an ESPR delegated act
/// carries ESPR's.
///
/// # Not a verdict on a fetched passport
///
/// Like [`Passport::validate`], this states what a record must meet to be
/// written. A passport fetched from another operator is signed and cannot be
/// changed by anyone; judge it by its signature.
#[must_use]
pub fn check_personal_data(passport: &Passport) -> Vec<FieldError> {
    let key = passport.product_group.catalog_key();
    let marks = default_catalog()
        .current_schema_version(key)
        .and_then(|version| personal_data_marks(key, version))
        .unwrap_or_default();

    let mut errors = Vec::new();

    if let Some(data) = &passport.product_group_data {
        let instance = product_group_data_instance(data);
        for field in &marks {
            let segments: Vec<&str> = field.split('.').collect();
            let mut values = Vec::new();
            values_at(&instance, &segments, &mut values);
            if values.iter().any(|v| holds_something(v))
                && !passport.personal_data.contains_key(field)
            {
                errors.push(FieldError {
                    field: format!("/productGroupData/{}", segments.join("/")),
                    message: format!(
                        "'{field}' is marked as able to hold personal data and has a value, so \
                         personalData must state what is held about it: nothing, or a record \
                         held outside the passport and the basis it is held on"
                    ),
                });
            }
        }
    }

    let conditions = governing_conditions(passport, key);
    for (field, statement) in &passport.personal_data {
        let pointer = format!(
            "/personalData/{}",
            field.replace('~', "~0").replace('/', "~1")
        );
        if !marks.contains(field) {
            errors.push(FieldError {
                field: pointer.clone(),
                message: format!(
                    "'{field}' is not a field this product group's schema marks as able to hold \
                     personal data, and a statement can only be made about one that is"
                ),
            });
        }
        let PersonalDataStatement::HeldOutside(held) = statement else {
            continue;
        };
        if held.record.as_str().trim().is_empty() {
            errors.push(FieldError {
                field: format!("{pointer}/record"),
                message: format!(
                    "personal data related to '{field}' is said to be held outside the passport, \
                     but no record is named"
                ),
            });
        }
        for condition in &conditions {
            if !condition.admits(held.lawful_basis) {
                errors.push(FieldError {
                    field: format!("{pointer}/lawfulBasis"),
                    message: format!(
                        "personal data related to '{field}' is held on the basis '{}', but {} \
                         admits customer personal data only with explicit consent",
                        wire_name(held.lawful_basis),
                        condition.provision
                    ),
                });
            }
        }
    }

    errors
}

/// A lawful basis as the operator wrote it, for a message that quotes it back.
fn wire_name(basis: LawfulBasis) -> String {
    serde_json::to_value(basis)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// The conditions on customer personal data set by the acts governing this
/// passport, one per distinct provision.
fn governing_conditions<'a>(passport: &Passport, key: &str) -> Vec<&'a CustomerPersonalData> {
    let catalog = default_instruments();
    let recorded: Vec<String> = if passport.applicable_instruments.is_empty() {
        catalog
            .instrument_refs_for(key)
            .into_iter()
            .map(|r| r.instrument)
            .collect()
    } else {
        passport
            .applicable_instruments
            .iter()
            .map(|r| r.instrument.clone())
            .collect()
    };
    let mut conditions: Vec<&CustomerPersonalData> = Vec::new();
    for id in &recorded {
        if let Some(condition) = catalog.customer_personal_data_for(id)
            && !conditions.contains(&condition)
        {
            conditions.push(condition);
        }
    }
    conditions
}

/// Every value at `segments` below `value`, descending into arrays wherever they
/// occur, since an array element sits at its array's path.
fn values_at<'a>(
    value: &'a serde_json::Value,
    segments: &[&str],
    out: &mut Vec<&'a serde_json::Value>,
) {
    if let serde_json::Value::Array(items) = value {
        for item in items {
            values_at(item, segments, out);
        }
        return;
    }
    match segments.split_first() {
        None => out.push(value),
        Some((first, rest)) => {
            if let Some(child) = value.get(*first) {
                values_at(child, rest, out);
            }
        }
    }
}

/// Whether a value says anything: not null, not blank text, and not a container
/// of only those.
fn holds_something(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::String(text) => !text.trim().is_empty(),
        serde_json::Value::Array(items) => items.iter().any(holds_something),
        serde_json::Value::Object(map) => map.values().any(holds_something),
        serde_json::Value::Bool(_) | serde_json::Value::Number(_) => true,
    }
}
