//! [`redact_passport`] — the audience-filtered view of a whole passport.
//!
//! # Why this is not a method on `Passport`
//!
//! It was one, and that is how the leak happened. `Passport` sits below this
//! module on the tier ladder, so an inherent `redact` could not reach
//! [`filter_by_audience`] without pointing an import down the ladder. It
//! therefore grew its own redaction — a second implementation of a
//! compliance-critical rule, with nothing proving the two agreed. They did not:
//! the copy served `seal`, `publicJwsSignature` and `disclosureSignatures` to
//! every audience, because those had no disclosure class and absent defaulted to
//! public.
//!
//! Redaction is the access layer's job. Putting it here lets it use the one
//! filter, and the ladder now enforces that rather than merely suggesting it.

use crate::disclosure::Audience;
use crate::passport::{PASSPORT_PROOF_FIELDS, Passport, PassportView};

use super::{DocumentScope, ProductGroupAccessPolicy, filter_by_audience};

/// Return an audience-filtered JSON view of `passport`.
///
/// Three rules, in order:
///
/// 1. **Proofs never travel in a view.** Every key in
///    [`PASSPORT_PROOF_FIELDS`] is removed for every audience, without
///    exception. A view is a payload; whoever serves it attaches the one
///    proof that covers exactly the bytes being sent. See that constant for
///    why this is not expressible as a disclosure class.
/// 2. **Envelope fields follow [`crate::disclosure::PASSPORT_FIELD_DISCLOSURE`]**, applied
///    through the shared scope-aware filter so a product group's schema can
///    never reclassify an envelope field by declaring a property of the same
///    name. The one exception is explicit: a schema may open a field named in
///    [`crate::disclosure::GROUP_OPENABLE_ENVELOPE_FIELDS`] to the public, and
///    only at the top-level envelope key.
/// 3. **Product-group data follows the policy for *this passport's* schema
///    version**, not the catalog's current one.
/// 4. **A statement about personal data goes where its field goes, and never
///    to the public.** Each entry of `personalData` is about one field of
///    `productGroupData`, and is kept only for an audience that may see that
///    field. No single class could express that, since the fields differ in
///    class. The public never receives one, because even a statement that
///    nothing is held tells a reader whether personal data about one item
///    exists. See [`crate::personal_data`].
///
/// # Why there is no `catalog` parameter
///
/// There used to be, and it resolved the policy through the catalog's single
/// unversioned disclosure map. A passport's signatures are frozen over the
/// redaction that produced them, so filtering by whatever that map says
/// *today* applies rules that may postdate the signature: the served body and
/// its proof then disagree for reasons no reader can distinguish from
/// tampering, and one reclassification breaks verification for every
/// already-published passport at once.
///
/// The passport carries its own [`schema_version`](Passport::schema_version), so
/// the correct version is never the caller's to supply — and with no
/// parameter there is no way to supply the wrong one.
///
/// # Failing closed
///
/// An unknown product group, or a schema version this build does not carry,
/// resolves to no policy. Product-group data is then reduced to its
/// `productGroup` tag for **every** audience rather than served unfiltered:
/// without per-field classes this crate cannot tell which fields are safe to
/// expose, so it exposes none. Envelope fields still apply, because they are
/// version-independent.
#[must_use]
pub fn redact_passport(passport: &Passport, audience: Audience) -> PassportView {
    let value = match serde_json::to_value(passport) {
        Ok(v) => v,
        Err(_) => return PassportView(serde_json::Value::Null),
    };

    // The whole-passport policy at the version this record was validated
    // against: the product group's own per-field tiers and envelope openings,
    // over the envelope defaults. `None` is the fail-closed signal below.
    //
    // Composed by `for_passport` rather than here. The two were composed
    // separately, and a composition done twice is one that can disagree with
    // itself — this view and a credential issued from the same passport would
    // then disclose different fields.
    let product_group_key = passport.product_group.catalog_key();
    let resolved =
        ProductGroupAccessPolicy::for_passport(product_group_key, &passport.schema_version);
    let policy = resolved
        .clone()
        .unwrap_or_else(ProductGroupAccessPolicy::passport_default);

    let mut view = filter_by_audience(&value, &policy, audience).filtered_data;

    if let Some(obj) = view.as_object_mut() {
        // Rule 1. Unconditional, and last, so nothing above can reintroduce
        // one by classing it Public.
        for proof in PASSPORT_PROOF_FIELDS {
            obj.remove(*proof);
        }

        // `product_group_data` is `Option` with no skip, so `None` serialises
        // as an explicit `null`. A view should not carry a key whose value is
        // "there is nothing here" — drop it rather than serve a null.
        if obj
            .get("productGroupData")
            .is_some_and(serde_json::Value::is_null)
        {
            obj.remove("productGroupData");
        }

        // Rule 3's fail-closed half. Keyed on the *policy*, not on whether
        // the catalog knows the product group: a known group at an unknown
        // schema version resolves to no policy, and a group-only check would
        // wave it through with every field public.
        match resolved {
            None => {
                if let Some(product_group_data) = obj.get("productGroupData")
                    && let Some(tag) = product_group_data.get("productGroup").cloned()
                {
                    obj.insert(
                        "productGroupData".to_owned(),
                        serde_json::json!({ "productGroup": tag }),
                    );
                }
            }
            Some(ref product_group_policy) => {
                drop_unclassified_product_group_keys(obj, product_group_policy);
            }
        }

        // Rule 4. Rebuilt from the passport rather than from what the filter
        // kept, so the envelope class the filter applied can only ever have
        // removed statements, never decided which ones survive.
        obj.remove("personalData");
        if audience != Audience::Public
            && resolved.is_some()
            && let Some(visible) = visible_statements(passport, &policy, audience)
        {
            obj.insert("personalData".to_owned(), visible);
        }
    }

    PassportView(view)
}

/// The passport's personal-data statements that `audience` may see, as JSON, or
/// `None` when it may see none.
///
/// A statement is kept when the audience may see every key on the way to its
/// field: `productGroupData` itself, classified as an envelope field, and then
/// each segment of the field's dotted path inside it. That is the walk
/// [`filter_by_audience`] makes, so a statement survives exactly when the field
/// it describes would.
fn visible_statements(
    passport: &Passport,
    policy: &ProductGroupAccessPolicy,
    audience: Audience,
) -> Option<serde_json::Value> {
    let visible: serde_json::Map<String, serde_json::Value> = passport
        .personal_data
        .iter()
        .filter(|(field, _)| {
            let mut path = vec!["productGroupData"];
            path.extend(field.split('.'));
            (1..=path.len()).all(|depth| {
                let scope = if depth == 1 {
                    DocumentScope::Envelope
                } else {
                    DocumentScope::ProductGroupData
                };
                audience.may_see(policy.disclosure_for_path(&path[..depth], scope))
            })
        })
        .filter_map(|(field, statement)| {
            serde_json::to_value(statement)
                .ok()
                .map(|value| (field.clone(), value))
        })
        .collect();
    (!visible.is_empty()).then_some(serde_json::Value::Object(visible))
}

/// Drop any `productGroupData` key the passport's **declared schema version**
/// does not declare.
///
/// # The hazard this closes
///
/// Sourcing classes from the version a passport was validated against is what
/// keeps a published passport filtered by the rules that produced its
/// signatures. The corollary is the danger: a key that version does not declare
/// is classified by nobody, and an unclassified key falls to the policy default
/// — `Public`. The version-pinned policy is safer than the catalog's single map
/// in every other respect and *less* safe in exactly this one, because it
/// under-applies where the map over-applied. **Under-applying disclosure is a
/// leak.**
///
/// Raising `default_disclosure` does not work: this policy covers the whole
/// document, and the envelope's public fields — `productName`, `status` — are
/// declared in no product group schema, so they would all vanish. The guard has
/// to be structural and scoped to `productGroupData`: where nobody has
/// classified the content, keep only what is accounted for.
///
/// Reaching it needs an **invalid** passport, since every product group schema
/// sets `additionalProperties: false` and validation rejects undeclared keys.
/// This is defence in depth. It runs for **every** audience, because an
/// unclassified field is not more disclosable to a credentialed reader than to
/// an anonymous one.
fn drop_unclassified_product_group_keys(
    view: &mut serde_json::Map<String, serde_json::Value>,
    product_group_policy: &ProductGroupAccessPolicy,
) {
    let Some(product_group_data) = view
        .get_mut("productGroupData")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };

    // `productGroup` is the variant tag rather than a schema property, and the
    // document has to keep round-tripping through `ProductGroupData`.
    //
    // Matching is by path suffix for the same reason the filter matches that
    // way: `field_disclosure` keys nested properties under their parent, so a
    // top-level key is accounted for when any policy key ends with it.
    product_group_data.retain(|key, _| {
        key == "productGroup"
            || product_group_policy
                .field_disclosure
                .keys()
                .any(|declared| declared == key || declared.ends_with(&format!(".{key}")))
    });
}
