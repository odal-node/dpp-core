//! ProductGroup access policy types and disclosure-class lookup.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{Disclosure, PASSPORT_FIELD_DISCLOSURE, ProductGroupCatalog};

/// Maps JSON field names to their disclosure class.
///
/// Fields not listed fall back to [`Self::default_disclosure`]. Matching is by
/// **normalized leaf key name** (case- and separator-insensitive), so a policy
/// key `disassemblyInstructions` also covers a payload key
/// `disassembly_instructions` — closing the casing/nesting drift that let
/// restricted fields leak to the public audience.
///
/// **Caution — leaf matching is path-insensitive.** A policy key matches that
/// leaf *wherever* it appears, at any depth. Do **not** restrict a generic leaf
/// name shared across objects (e.g. `name`, `value`, `country`, `address`): such
/// a key would redact `facility.address` *and* `manufacturer.address` alike,
/// over-redacting Annex III public fields. Use only specific, unambiguous field
/// names (e.g. `dueDiligenceUrl`, `svhcSubstances`). Gating a shared leaf on a
/// single path would require making the matcher path-aware first.
///
/// A schema cannot get this wrong quietly: `access::tests`'
/// `no_schema_declares_one_field_name_in_two_classes` fails the build if one
/// name is declared in two classes. It is a real limit rather than an
/// oversight, and it decides how a nested field may be classified — a nested
/// property sharing a leaf name with a more permissive field elsewhere in the
/// same schema cannot be restricted on the leaf, because that would restrict
/// the twin as well. Where that arises the **enclosing object** carries the
/// restriction and [`super::filter::filter_by_audience`] removes the whole
/// subtree before reaching the leaf, which is why it is a constraint on
/// expression rather than a hole in enforcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductGroupAccessPolicy {
    /// Human-readable policy name (e.g., `"textile-v1.1"`).
    pub name: String,
    /// The product group this policy applies to.
    pub product_group: String,
    /// Map of JSON field name → disclosure class. A listed field is matched
    /// wherever it appears in the document (any nesting depth), by normalized key.
    pub field_disclosure: HashMap<String, Disclosure>,
    // NOTE: `ProductGroupAccessPolicy::from_schema` below reads the same classes out
    // of a *versioned* schema. Prefer it — see its doc comment.
    /// Class applied to fields **not** listed in `field_disclosure`. Defaults
    /// to `Public` (backward-compatible: only restricted fields need listing).
    /// Set to a non-public class for a true default-deny (fail-closed) policy,
    /// where every public field must be explicitly listed as `Public`.
    #[serde(default = "disclosure_public")]
    pub default_disclosure: Disclosure,
}

fn disclosure_public() -> Disclosure {
    Disclosure::Public
}

/// Whether `a` and `b` are equal for field-matching purposes once both are
/// normalized — non-alphanumerics (`_`, `-`) dropped, case-folded, so
/// `disassemblyInstructions` == `disassembly_instructions` — without
/// allocating a `String` for either side. [`ProductGroupAccessPolicy::disclosure_for_field`]
/// runs this once per classified field, per document key, at every
/// recursion depth of [`super::filter::filter_by_audience`], so avoiding an
/// allocation per comparison matters there.
fn keys_match_normalized(a: &str, b: &str) -> bool {
    let mut a_chars = a.chars().filter(char::is_ascii_alphanumeric);
    let mut b_chars = b.chars().filter(char::is_ascii_alphanumeric);
    loop {
        match (a_chars.next(), b_chars.next()) {
            (Some(x), Some(y)) if x.eq_ignore_ascii_case(&y) => {}
            (None, None) => return true,
            _ => return false,
        }
    }
}

/// Universal conformity-evidence fields present on every published passport
/// payload (signatures, audit trails). Folded into each product group's policy so they
/// are not repeated in every manifest.
const COMMON_CONFORMITY: &[&str] = &[
    "jwsSignature",
    "complianceReport",
    "auditHistory",
    "supplyChainTrace",
];

/// Parse one `x-disclosure` token. `None` for anything outside the four classes.
fn parse_disclosure(token: &str) -> Option<Disclosure> {
    match token {
        "public" => Some(Disclosure::Public),
        "restricted" => Some(Disclosure::Restricted),
        "conformity" => Some(Disclosure::Conformity),
        "individual" => Some(Disclosure::Individual),
        _ => None,
    }
}

/// Walk every `properties` map in a schema and record each declared class.
///
/// Descends through nested `properties`, array `items`, `additionalProperties`,
/// the `definitions` / `$defs` blocks that `$ref` resolves into, and the
/// `allOf` / `anyOf` / `oneOf` combinators.
///
/// `$ref` itself is deliberately **not** followed. Matching is by leaf name, so
/// a definition's properties are reached by walking the definitions block
/// directly; following the pointer as well would visit them twice and buy
/// nothing. It also means a cyclic `$ref` cannot loop this function.
///
/// A leaf name declared twice with **different** classes keeps the more
/// restrictive one. That is a fail-closed tie-break for a schema that should
/// not exist — `access::tests` rejects the ambiguity at build time — and it is
/// never reached by a schema that passes that gate.
fn collect_disclosures(node: &serde_json::Value, out: &mut HashMap<String, Disclosure>) {
    let Some(object) = node.as_object() else {
        return;
    };

    if let Some(properties) = object.get("properties").and_then(|p| p.as_object()) {
        for (name, prop) in properties {
            if let Some(class) = prop
                .get("x-disclosure")
                .and_then(serde_json::Value::as_str)
                .and_then(parse_disclosure)
            {
                out.entry(name.clone())
                    .and_modify(|existing| *existing = existing.most_restrictive(class))
                    .or_insert(class);
            }
            collect_disclosures(prop, out);
        }
    }

    for key in ["items", "additionalProperties"] {
        if let Some(child) = object.get(key) {
            collect_disclosures(child, out);
        }
    }

    for key in ["definitions", "$defs"] {
        if let Some(block) = object.get(key).and_then(|b| b.as_object()) {
            for definition in block.values() {
                collect_disclosures(definition, out);
            }
        }
    }

    for key in ["allOf", "anyOf", "oneOf"] {
        if let Some(branches) = object.get(key).and_then(|b| b.as_array()) {
            for branch in branches {
                collect_disclosures(branch, out);
            }
        }
    }
}

impl ProductGroupAccessPolicy {
    /// The policy for the schema version a passport was validated against.
    ///
    /// **The constructor to reach for.** A passport stores its
    /// `schema_version`, so passing that here filters it by the same disclosure
    /// classes that produced its frozen signatures — for the life of the
    /// passport, whatever the current version later says. See
    /// [`Self::from_schema`] for why that matters.
    ///
    /// Returns `None` when the product group or version is unknown to the registry, so
    /// an unrecognised pair fails closed rather than serving an all-public view.
    /// Takes the version as a string so a caller holding a `Passport` can pass
    /// `passport.schema_version` directly, and so consumers need no `semver`
    /// dependency of their own. An unparseable version yields `None`, the same
    /// as an unknown one — both fail closed.
    #[must_use]
    pub fn for_schema_version(product_group_key: &str, version: &str) -> Option<Self> {
        let parsed: semver::Version = version.parse().ok()?;
        let registry = crate::schemas::VersionedSchemaRegistry::new();
        let json = registry.get(product_group_key, &parsed)?;
        Self::from_schema(product_group_key, version, json)
    }

    /// Build the policy from the catalog's single, unversioned disclosure map.
    ///
    /// **Deprecated in favour of [`Self::for_schema_version`].** The catalog
    /// carries one map against many schema versions, so a passport served
    /// through this constructor is filtered by whatever the map says *today* —
    /// not by what it said when the passport's signatures were frozen. A
    /// reclassification therefore breaks verification for every already-published
    /// passport at once.
    ///
    /// Retained rather than removed: it is the only constructor that answers
    /// "what does the current build consider public", which is a legitimate
    /// question for tooling that is not serving a specific passport.
    #[deprecated(
        since = "0.17.0",
        note = "use `for_schema_version` — the catalog map is unversioned, so it \
                filters published passports by rules that may postdate their signatures"
    )]
    pub fn from_catalog(catalog: &ProductGroupCatalog, product_group_key: &str) -> Option<Self> {
        let descriptor = catalog.get(product_group_key)?;
        let mut field_disclosure: HashMap<String, Disclosure> = descriptor.disclosure.clone();
        for field in COMMON_CONFORMITY {
            field_disclosure
                .entry((*field).to_owned())
                .or_insert(Disclosure::Conformity);
        }
        Some(Self {
            name: format!("{product_group_key}-{}", descriptor.current_schema_version),
            product_group: product_group_key.to_owned(),
            field_disclosure,
            default_disclosure: Disclosure::Public,
        })
    }

    /// Build the policy from a **specific schema version's** own `x-disclosure`
    /// annotations, rather than from the catalog's single unversioned map.
    ///
    /// # Why this exists
    ///
    /// A passport's signatures are frozen at publish; the bytes each one covers
    /// are produced at *serve* time by whichever policy is in force then. The
    /// catalog carries **one** disclosure map against **many** schema versions,
    /// so the day a delegated act reclassifies a field — `restricted` →
    /// `public` is the move these acts make — the view served for an
    /// already-published passport gains a field its frozen signature never
    /// covered. Verification then fails for every affected passport at once and
    /// is indistinguishable from tampering.
    ///
    /// A schema version, by contrast, is already stored **on the passport** and
    /// already authoritative for reads (`resolve_schema_version` returns the
    /// stored one). Reading disclosure from there means a passport is always
    /// filtered by the same classes that produced its signature, permanently,
    /// with no extra field and no new machinery. A reclassification becomes a
    /// new schema version — which is correct rather than costly: changing who
    /// may see a field changes what the document means.
    ///
    /// # Why the annotation lives on the property
    ///
    /// Co-location. A field and its access class are declared in the same
    /// object, so a field cannot be added without its disclosure slot appearing
    /// in the same diff — and `every_property_declares_a_valid_disclosure_class`
    /// turns that into a build-time guarantee. The catalog map has no such
    /// property: a field added without an entry silently defaults to public,
    /// which for Annex XIII point 2, 3 or 4 content is a leak.
    ///
    /// # Every depth, not only the top level
    ///
    /// Annotations are collected from **every** `properties` map in the schema —
    /// nested objects, array `items`, and the `definitions`/`$defs` blocks that
    /// `$ref` pulls in — not just the root one.
    ///
    /// Reading only the root was a silent hole: [`super::filter::filter_by_audience`]
    /// classifies keys at every nesting depth, so a nested property fell to
    /// `default_disclosure` — `Public` — no matter what it declared. A field
    /// annotated `restricted` in the place this doc comment tells an author to
    /// put it was served to anyone, and neither build-time gate looked deep
    /// enough to notice. Nothing leaked, because every nested property then in
    /// the tree sat under a public parent and was correctly public anyway; the
    /// defect was that annotating one correctly would not have helped.
    ///
    /// Returns `None` if the schema is absent or is not a JSON object with
    /// `properties`. An unparseable schema must not silently produce an
    /// all-public policy.
    #[must_use]
    pub fn from_schema(product_group_key: &str, version: &str, schema_json: &str) -> Option<Self> {
        let schema: serde_json::Value = serde_json::from_str(schema_json).ok()?;
        // The root `properties` map is still required: a schema without one is
        // not a shape this policy can describe, and guessing is how an
        // unparseable product group ends up all-public.
        schema.get("properties")?.as_object()?;

        let mut field_disclosure: HashMap<String, Disclosure> = HashMap::new();
        collect_disclosures(&schema, &mut field_disclosure);

        for field in COMMON_CONFORMITY {
            field_disclosure
                .entry((*field).to_owned())
                .or_insert(Disclosure::Conformity);
        }

        Some(Self {
            name: format!("{product_group_key}-{version}"),
            product_group: product_group_key.to_owned(),
            field_disclosure,
            default_disclosure: Disclosure::Public,
        })
    }

    /// Default access policy for top-level passport fields (product group-agnostic).
    ///
    /// **Invariant — no mutable-after-publish *compliance content* may sit at
    /// `Public`.** The public view is what a passport's public signature is
    /// computed over, so `Public` content that changes after publish makes the
    /// served body stop verifying against its own signature. Content that must
    /// stay re-writable post-publish is therefore classified *up*, out of the signed
    /// public payload — see `lintResult` below.
    ///
    /// **The exemption, stated so it is not read as an oversight.** Lifecycle
    /// metadata — `status`, `publishedAt`, `updatedAt`, `qrCodeUrl` — is `Public`
    /// *and* mutable after publish. That is consistent only because a conforming
    /// server serves the **signed payload**, not the live row: what it emits is
    /// frozen at publish time and therefore agrees with the attached signature by
    /// construction. A server that redacts the live row into a public view and
    /// attaches the publish-time proof to it reintroduces exactly the divergence
    /// this invariant exists to prevent, for these fields and any future one.
    pub fn passport_default() -> Self {
        let mut field_disclosure = HashMap::new();
        for (field, class) in PASSPORT_FIELD_DISCLOSURE {
            field_disclosure.insert((*field).to_owned(), *class);
        }
        Self {
            name: "passport-v1.0".into(),
            product_group: "passport".into(),
            field_disclosure,
            default_disclosure: Disclosure::Public,
        }
    }

    /// Get the disclosure class for a field, matched by normalized key name
    /// (case/separator-insensitive). Unlisted fields fall back to
    /// `default_disclosure`.
    ///
    /// **Deterministic when more than one key matches.** `field_disclosure` is
    /// keyed by the literal name but matched after normalization, so two
    /// distinct keys — `jwsSignature` and `jws_signature` — can both answer one
    /// lookup. Taking the first match meant taking whichever one `HashMap`
    /// iteration reached first, which is unspecified and reseeded per map: the
    /// same passport, policy and audience could be answered `Conformity` on one
    /// call and `Public` on the next, in one process. A disclosure verdict that
    /// varies per map instance also makes the served field set unstable, which
    /// is fatal for content-binding.
    ///
    /// So every match is considered and the **most restrictive** wins. Ambiguity
    /// resolves the safe way, and it resolves the same way every time.
    pub fn disclosure_for_field(&self, field_name: &str) -> Disclosure {
        self.field_disclosure
            .iter()
            .filter(|(k, _)| keys_match_normalized(k, field_name))
            .map(|(_, d)| *d)
            .reduce(Disclosure::most_restrictive)
            .unwrap_or(self.default_disclosure)
    }
}
