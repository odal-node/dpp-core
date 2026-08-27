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
/// **Keys are path suffixes, and the most specific match wins.** A key may name
/// a bare leaf (`svhcSubstances`) or a path (`anodeMaterial.weightPct`). A path
/// key applies only where those segments end the field's own path, so one leaf
/// name can carry different classes in different places.
///
/// That used to be impossible. Keyed by leaf alone, a name had to mean one thing
/// everywhere: battery's `materialComposition` and `criticalRawMaterial` both
/// declare `name` and `casNumber`, so classifying one of those positions
/// classified its twin, and the fail-closed tie-break resolved any disagreement
/// to the restrictive class — redacting Annex III content the public passport is
/// legally required to carry. **Over-redaction is not the safe direction when
/// the public view is an obligation.** The workaround was to restrict the
/// enclosing object instead and let the filter remove the whole subtree, which
/// worked — battery still does it, which is why nothing was ever mis-served —
/// but meant a nested field could not be classified on its own terms.
///
/// Both of those are `definitions` blocks reached by `$ref`, as is every shared
/// leaf in this crate's schemas. Path keys alone did not reach them: a
/// definition was walked at the root and kept bare-leaf keys, so the collision
/// survived. Following the pointer is what made the distinction expressible —
/// see `collect_disclosures`.
///
/// A one-segment key still matches at any depth, because a bare leaf is simply
/// the weakest suffix rather than a special case — so every policy written
/// before paths existed behaves exactly as it did.
///
/// Scope is a separate and still-enforced boundary: a class drawn from a product
/// group's schema does not reach the passport envelope however precisely it is
/// written. See [`DocumentScope`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductGroupAccessPolicy {
    /// Human-readable policy name (e.g., `"textile-v1.1"`).
    pub name: String,
    /// The product group this policy applies to.
    pub product_group: String,
    /// Map of JSON field name → disclosure class, **scoped to the product
    /// group's own data**.
    ///
    /// Built from the product group's schema, so its keys describe the contents
    /// of `productGroupData` and nothing else. It is consulted only for keys at
    /// or below that field — see [`DocumentScope`].
    ///
    /// It used to be consulted for *every* key at every depth, which meant a
    /// product group's classes were applied to the passport envelope as well.
    /// A field on the envelope whose nested key name happened to match a schema
    /// property inherited that property's class: an envelope timestamp named
    /// `recordedAt` was stripped from every public battery projection, because
    /// the battery schema declares its own `recordedAt` as individual-tier data.
    /// The names come from a schema that never described the envelope, so
    /// applying them there was never right.
    pub field_disclosure: HashMap<String, Disclosure>,
    /// Map of JSON field name → disclosure class that applies **anywhere** in
    /// the document.
    ///
    /// Envelope fields and universal conformity evidence: names that are not a
    /// product group's to define, and that mean the same thing wherever they
    /// appear. A signature is conformity evidence at any depth.
    #[serde(default)]
    pub envelope_disclosure: HashMap<String, Disclosure>,
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

/// Where in a passport document a key sits, which decides whose classes apply.
///
/// A passport is two documents in one envelope: the fields every passport has,
/// and one product group's payload under `productGroupData`. Their field names
/// come from different places and are governed by different authorities — the
/// envelope's by this crate, the payload's by that product group's schema — and
/// nothing stops the two from choosing the same word.
///
/// They did. A schema-derived class was applied to any key of that name at any
/// depth, so an envelope field called `recordedAt` was withheld from the public
/// because *battery* declares a `recordedAt` as individual-tier data. The
/// document was then unreadable, which is how it was noticed; the same collision
/// on a field the reader does not immediately need would simply have gone
/// missing.
///
/// Deliberately a two-value scope rather than a full path matcher. The claim
/// being enforced is only that a product group's vocabulary stops at its own
/// payload — which is where the schema's authority stops — and a boundary
/// nobody has to write paths for cannot be written wrongly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DocumentScope {
    /// A passport envelope field. Only universally-scoped classes apply.
    Envelope,
    /// At or below `productGroupData`, where the product group's schema governs.
    ProductGroupData,
}

/// How many segments of `policy_key` match the tail of `path`, or `None` if it
/// is not a suffix of it at all.
///
/// A policy key is a dotted path (`materialComposition.name`) or a bare leaf
/// (`name`, which is the one-segment case). It matches when its segments align
/// with the last segments of `path`, each compared by [`keys_match_normalized`].
/// The returned count is the specificity — a longer match is a more precise
/// statement about where the field sits, and wins.
///
/// Suffix rather than full path so a class stays attached to the shape it
/// describes rather than to one position in one schema. A definition reused
/// under two parents keeps its meaning, and a policy written before nesting
/// existed keeps working.
///
/// # The leaf fallback, and why it is specificity zero
///
/// A caller that knows only a leaf name — [`ProductGroupAccessPolicy::disclosure_for_field`]
/// is the public one — cannot supply the path a nested key was recorded under.
/// Without a fallback, asking about `recordedAt` when the policy holds
/// `usageHistory.recordedAt` would find no match and return the default, and the
/// default is `Public`. **That is fail-open for restricted data**, reached by
/// asking an under-specified question rather than by any schema being wrong.
///
/// So a policy key longer than the query still matches on its final segment,
/// scoring `0`. Being the weakest score, it loses to every genuine suffix match,
/// and ties among leaf matches resolve to the most restrictive class. A question
/// that cannot distinguish two positions gets the conservative answer for both,
/// while the filter — which always has the real path — gets the precise one.
fn path_suffix_depth(policy_key: &str, path: &[&str]) -> Option<usize> {
    let key_segments = policy_key.split('.').count();

    if key_segments > path.len() {
        // Fall back to the leaf. `path` is never empty in practice — the filter
        // always pushes a key before classifying — but an empty one has no leaf
        // to compare and must not match anything.
        let (Some(key_leaf), Some(query_leaf)) = (policy_key.split('.').next_back(), path.last())
        else {
            return None;
        };
        return keys_match_normalized(key_leaf, query_leaf).then_some(0);
    }

    let tail = &path[path.len() - key_segments..];
    policy_key
        .split('.')
        .zip(tail.iter())
        .all(|(k, p)| keys_match_normalized(k, p))
        .then_some(key_segments)
}

/// Whether `a` and `b` are equal for field-matching purposes once both are
/// normalized — non-alphanumerics (`_`, `-`) dropped, case-folded, so
/// `disassemblyInstructions` == `disassembly_instructions` — without
/// allocating a `String` for either side. [`ProductGroupAccessPolicy::disclosure_for_path`]
/// runs this once per policy key, per document key, at every recursion depth of
/// [`super::filter::filter_by_audience`], so avoiding an allocation per
/// comparison matters there.
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

/// [`COMMON_CONFORMITY`] as a map, for the universally-scoped half of a policy.
fn common_conformity() -> HashMap<String, Disclosure> {
    COMMON_CONFORMITY
        .iter()
        .map(|field| ((*field).to_owned(), Disclosure::Conformity))
        .collect()
}

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

/// Record `class` for `key`, keeping the more restrictive of two declarations.
fn record_disclosure(out: &mut HashMap<String, Disclosure>, key: String, class: Disclosure) {
    out.entry(key)
        .and_modify(|existing| *existing = existing.most_restrictive(class))
        .or_insert(class);
}

/// Resolve a local `$ref` — `#/definitions/x`, `#/$defs/x` — against the schema
/// root. Non-local and unresolvable pointers yield `None`.
///
/// Only same-document pointers are followed. A `$ref` to another file names a
/// document this crate does not have, and inventing a class for a shape it
/// cannot read is worse than leaving the definition's bare-leaf key to cover it.
fn resolve_local_ref<'a>(
    node: &serde_json::Map<String, serde_json::Value>,
    root: &'a serde_json::Value,
) -> Option<(&'a serde_json::Value, String)> {
    let pointer = node.get("$ref").and_then(serde_json::Value::as_str)?;
    let path = pointer.strip_prefix('#')?;
    let target = root.pointer(path)?;
    Some((target, pointer.to_owned()))
}

/// Walk every `properties` map in a schema and record each declared class.
///
/// Descends through nested `properties`, array `items`, `additionalProperties`,
/// the `definitions` / `$defs` blocks, the `allOf` / `anyOf` / `oneOf`
/// combinators, and local `$ref` pointers.
///
/// # Keys are paths, not bare leaf names
///
/// A property nested at `materialComposition.name` is recorded under that
/// dotted path rather than under `name`. Recording the leaf alone meant one name
/// carried one class for the whole document, and collapsing a collision took the
/// more restrictive class for both — over-redacting Annex III content the public
/// view is required to carry. Over-redaction is not the safe direction here.
///
/// Only `properties` adds a segment. Array `items` and the `allOf` / `anyOf` /
/// `oneOf` combinators describe the *same* position as their parent, so they add
/// none — which matches the filter, where an array index is not a path segment
/// either.
///
/// # A definition is recorded twice, deliberately
///
/// `definitions` / `$defs` blocks are walked at the **empty path**, giving their
/// properties bare-leaf keys, *and* every local `$ref` is followed so the same
/// properties are recorded again under the path of whatever referred to them.
/// Battery's `materialComposition` is `$ref`'d from `anodeMaterial`,
/// `cathodeMaterial` and `electrolyteMaterial`, so its `name` lands as
/// `anodeMaterial.name`, `cathodeMaterial.name`, `electrolyteMaterial.name` —
/// and as bare `name`.
///
/// The two serve different purposes and neither replaces the other.
///
/// The **referring paths** are what make a definition's class positional. Two
/// definitions declaring the same leaf now yield distinct keys, so
/// `criticalRawMaterials.name` can be `Public` while `anodeMaterial.name` is
/// `Restricted`, which by-leaf keying could not express: the collision merged to
/// the restrictive class and redacted both.
///
/// The **bare-leaf key is the floor**, and dropping it would be a leak. This walk
/// descends `properties`, `items`, `additionalProperties`, the three combinators
/// and `$ref`, and nothing else — not `patternProperties`, `if` / `then` /
/// `else`, `not`, or `dependentSchemas`. A definition reached only from one of
/// those would have no referring path recorded at all, and with no bare-leaf key
/// its class would vanish and the field would fall to `default_disclosure`,
/// which is `Public`. Keeping the leaf means an unreachable reference
/// over-applies rather than under-applies. Since a referring path always scores
/// higher than the one-segment leaf, the floor never overrides a position that
/// was named explicitly.
///
/// A cyclic `$ref` cannot loop this function: `active_refs` holds the pointers
/// on the current descent and a pointer already there is not re-entered.
///
/// A path declared twice with **different** classes keeps the more restrictive
/// one — the fail-closed tie-break, now reachable mainly where one definition is
/// referred to from two places whose own classes differ.
fn collect_disclosures(
    node: &serde_json::Value,
    path: &str,
    root: &serde_json::Value,
    active_refs: &mut Vec<String>,
    out: &mut HashMap<String, Disclosure>,
) {
    let Some(object) = node.as_object() else {
        return;
    };

    // A `$ref` describes the position its holder occupies, so the target is
    // walked at `path` unchanged — the same rule `items` follows, and why
    // `items: { $ref: ... }` lands a definition on the array's own path.
    if let Some((target, pointer)) = resolve_local_ref(object, root)
        && !active_refs.iter().any(|seen| seen == &pointer)
    {
        active_refs.push(pointer);
        collect_disclosures(target, path, root, active_refs, out);
        active_refs.pop();
    }

    if let Some(properties) = object.get("properties").and_then(|p| p.as_object()) {
        for (name, prop) in properties {
            let child_path = if path.is_empty() {
                name.clone()
            } else {
                format!("{path}.{name}")
            };
            if let Some(class) = prop
                .get("x-disclosure")
                .and_then(serde_json::Value::as_str)
                .and_then(parse_disclosure)
            {
                record_disclosure(out, child_path.clone(), class);
            }
            collect_disclosures(prop, &child_path, root, active_refs, out);
        }
    }

    // An array element sits at the same path its key does — the filter does not
    // treat an index as a segment either — so `items` carries `path` unchanged.
    if let Some(child) = object.get("items") {
        collect_disclosures(child, path, root, active_refs, out);
    }

    // A map's values sit one segment deeper than the map, under a key chosen by
    // the document rather than the schema. No static path can name that segment,
    // so carrying `path` here would record a key no document can match and the
    // field would fall to the `Public` default. These restart at the empty path
    // instead and keep bare-leaf keys, which match at any depth.
    if let Some(child) = object.get("additionalProperties") {
        collect_disclosures(child, "", root, active_refs, out);
    }

    // Walked at the root as the floor described above, independently of whether
    // any `$ref` above reached them.
    for key in ["definitions", "$defs"] {
        if let Some(block) = object.get(key).and_then(|b| b.as_object()) {
            for definition in block.values() {
                collect_disclosures(definition, "", root, active_refs, out);
            }
        }
    }

    // Combinators describe the same object their parent does, so they add no
    // segment.
    for key in ["allOf", "anyOf", "oneOf"] {
        if let Some(branches) = object.get(key).and_then(|b| b.as_array()) {
            for branch in branches {
                collect_disclosures(branch, path, root, active_refs, out);
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
        let field_disclosure: HashMap<String, Disclosure> = descriptor.disclosure.clone();
        Some(Self {
            name: format!("{product_group_key}-{}", descriptor.current_schema_version),
            product_group: product_group_key.to_owned(),
            field_disclosure,
            envelope_disclosure: common_conformity(),
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
        let mut active_refs: Vec<String> = Vec::new();
        collect_disclosures(
            &schema,
            "",
            &schema,
            &mut active_refs,
            &mut field_disclosure,
        );

        Some(Self {
            name: format!("{product_group_key}-{version}"),
            product_group: product_group_key.to_owned(),
            field_disclosure,
            envelope_disclosure: common_conformity(),
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
        let mut envelope_disclosure = common_conformity();
        for (field, class) in PASSPORT_FIELD_DISCLOSURE {
            envelope_disclosure.insert((*field).to_owned(), *class);
        }
        Self {
            name: "passport-v1.0".into(),
            product_group: "passport".into(),
            // No product group's schema is in play here, so there is nothing
            // this policy could say about a payload it cannot identify.
            field_disclosure: HashMap::new(),
            envelope_disclosure,
            default_disclosure: Disclosure::Public,
        }
    }

    /// Get the disclosure class for a key, given where in the document it sits.
    ///
    /// The scope decides which map is consulted, and that is the whole point:
    /// [`DocumentScope::Envelope`] never reaches the product group's schema
    /// classes, because those names describe `productGroupData` and were only
    /// ever meaningful there.
    ///
    /// Both scopes consult [`Self::envelope_disclosure`]. Those names — a
    /// signature, an audit trail — are not a product group's to define and mean
    /// the same thing at any depth, so a product-group payload cannot reclassify
    /// one by declaring a property with the same name.
    #[must_use]
    pub fn disclosure_for_key(&self, key: &str, scope: DocumentScope) -> Disclosure {
        self.disclosure_for_path(&[key], scope)
    }

    /// Get the disclosure class for a key, given the full path of object keys
    /// that leads to it.
    ///
    /// `path` is the chain of **object keys** from the document root to the key
    /// being classified, innermost last. Array indices are not segments — an
    /// element sits where its key sits — so
    /// `criticalRawMaterials[0].casNumber` is
    /// `["criticalRawMaterials", "casNumber"]`.
    ///
    /// # Most specific wins
    ///
    /// A policy key matches when its own segments are a **suffix** of `path`,
    /// compared with the same case- and separator-insensitive normalization used
    /// for bare names. The match with the most segments wins.
    ///
    /// That is what lets one leaf name carry different classes in different
    /// places. Battery's `materialComposition` and `criticalRawMaterial` both
    /// declare `name` and `casNumber`; keyed by leaf alone, one class had to
    /// cover both positions, and any disagreement resolved to the restrictive
    /// one — redacting Annex III content from the public view, which the
    /// regulation requires it to carry. Over-redaction is not the safe direction
    /// when the public passport is a legal obligation.
    ///
    /// Both are `definitions` blocks, so the paths that separate them
    /// (`anodeMaterial.name` against `criticalRawMaterials.name`) exist only
    /// because `collect_disclosures` follows `$ref`. A path matcher over a
    /// walk that stopped at the definitions block would have left this exact
    /// case where it was.
    ///
    /// A single-segment policy key still matches at any depth, because a
    /// one-segment suffix is the weakest match rather than a special case. Every
    /// policy written before paths existed therefore behaves exactly as it did.
    ///
    /// # Ties
    ///
    /// Two policy keys of equal specificity that both match resolve to the
    /// **most restrictive** class. `field_disclosure` is keyed by literal name
    /// but matched after normalization, so `jwsSignature` and `jws_signature` can
    /// both answer one lookup; taking whichever `HashMap` iteration reached first
    /// made a disclosure verdict vary between calls in one process, which is
    /// fatal for content-binding. Ambiguity resolves the safe way, and the same
    /// way every time.
    #[must_use]
    pub fn disclosure_for_path(&self, path: &[&str], scope: DocumentScope) -> Disclosure {
        let scoped = match scope {
            DocumentScope::ProductGroupData => Some(&self.field_disclosure),
            DocumentScope::Envelope => None,
        };

        let mut best: Option<(usize, Disclosure)> = None;
        for (policy_key, class) in scoped
            .into_iter()
            .chain(std::iter::once(&self.envelope_disclosure))
            .flat_map(|map| map.iter())
        {
            let Some(depth) = path_suffix_depth(policy_key, path) else {
                continue;
            };
            best = Some(match best {
                None => (depth, *class),
                Some((best_depth, _)) if depth > best_depth => (depth, *class),
                Some((best_depth, best_class)) if depth == best_depth => {
                    (best_depth, best_class.most_restrictive(*class))
                }
                Some(kept) => kept,
            });
        }

        best.map_or(self.default_disclosure, |(_, class)| class)
    }

    /// Get the disclosure class for a field, matched by normalized key name
    /// (case/separator-insensitive). Unlisted fields fall back to
    /// `default_disclosure`.
    ///
    /// Answers for a key **inside the product group's data**, which is the scope
    /// its classes were written for. Prefer [`Self::disclosure_for_key`] and say
    /// which scope you mean; this is kept because asking "what class does this
    /// product group give this field" is a legitimate question on its own.
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
    #[must_use]
    pub fn disclosure_for_field(&self, field_name: &str) -> Disclosure {
        self.disclosure_for_key(field_name, DocumentScope::ProductGroupData)
    }
}
