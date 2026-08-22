//! Schema upcast lenses: pure, versioned, deterministic `v_n → v_m` transforms
//! applied at *read time*.
//!
//! Signed passports are immutable; delegated acts are not. When a sector schema
//! gains a new version, existing signed records must stay byte-identical (their
//! signatures depend on it) yet remain consumable by new-version readers. A lens
//! transforms a record's sector data from the version it was written against up
//! to a newer one, **without touching the canonical signed original** — the
//! derived view carries honest provenance (`derived`, `lens_chain`, `lossy`) and
//! is never presented as the original signature.
//!
//! Only **upcast** (old → new) is supported: the past can read the future never.
//! Lenses are law-adjacent artifacts — each carries the regulatory change that
//! motivated it. They start as Rust impls compiled into core (versioned with the
//! schemas they bridge); an expression/bundle-delivered form can come later.

use std::collections::{HashMap, VecDeque};

use semver::Version;
use serde_json::Value;

/// A single-hop, pure upcast transform between two versions of one sector's
/// schema.
pub struct Lens {
    pub sector: String,
    pub from: Version,
    pub to: Version,
    /// Whether the transform may drop or default source information. An honest
    /// lens over a purely additive schema change is `false`; one that must
    /// discard a removed field is `true`.
    pub lossy: bool,
    /// The regulatory change or rationale this lens bridges.
    pub note: &'static str,
    /// Pure transform, total over inputs that validate against `from`.
    transform: fn(&Value) -> Result<Value, LensError>,
}

impl Lens {
    #[must_use]
    pub fn new(
        sector: impl Into<String>,
        from: Version,
        to: Version,
        lossy: bool,
        note: &'static str,
        transform: fn(&Value) -> Result<Value, LensError>,
    ) -> Self {
        Self {
            sector: sector.into(),
            from,
            to,
            lossy,
            note,
            transform,
        }
    }
}

/// A lens transform failed on its input. A well-formed lens over data that
/// validates against `from` never returns this; it exists so a transform can
/// refuse structurally impossible input rather than silently corrupt it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LensError(pub String);

impl std::fmt::Display for LensError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "lens transform failed: {}", self.0)
    }
}

/// A derived (upcast) view of sector data, with honest provenance. Never the
/// canonical signed original — `derived` is always `true`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DerivedView {
    /// The transformed sector data, conforming to the `to` schema.
    pub data: Value,
    /// Always `true`: this is a read-time derivation, not signed source.
    pub derived: bool,
    /// The version derived from, and the version now conformed to.
    pub from: String,
    pub to: String,
    /// The ordered hops applied — `[["1.0.0","2.0.0"]]` — for multi-hop chains.
    pub lens_chain: Vec<[String; 2]>,
    /// `true` if any hop in the chain dropped or defaulted information.
    pub lossy: bool,
}

/// Why an upcast could not be produced. Never a silent identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpcastError {
    /// No chain of registered lenses bridges `from` → `to` for this sector.
    NoPath {
        sector: String,
        from: Version,
        to: Version,
    },
    /// `to` is not newer than `from` — downcast is never supported.
    NotAnUpcast { from: Version, to: Version },
    /// A lens transform in the chain failed.
    Transform(LensError),
    /// A version string could not be parsed as semver.
    BadVersion(String),
}

impl std::fmt::Display for UpcastError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoPath { sector, from, to } => {
                write!(f, "no lens path for {sector} {from} → {to}")
            }
            Self::NotAnUpcast { from, to } => {
                write!(
                    f,
                    "{to} is not an upcast of {from} — downcast is unsupported"
                )
            }
            Self::Transform(e) => write!(f, "{e}"),
            Self::BadVersion(v) => write!(f, "'{v}' is not a valid semver version"),
        }
    }
}

impl std::error::Error for UpcastError {}

/// A registry of upcast lenses, composing single-hop transforms into multi-hop
/// chains at read time.
pub struct LensRegistry {
    lenses: Vec<Lens>,
}

impl LensRegistry {
    /// A registry pre-loaded with the compiled-in built-in lenses.
    #[must_use]
    pub fn new() -> Self {
        Self {
            lenses: builtin_lenses(),
        }
    }

    /// Build a registry from an explicit lens set (extensibility / tests).
    #[must_use]
    pub fn from_lenses(lenses: Vec<Lens>) -> Self {
        Self { lenses }
    }

    /// Upcast `data` for `sector` from `from` up to `to`, composing single-hop
    /// lenses along the fewest-hop path.
    ///
    /// `from == to` is the identity (a no-loss derived view of the same version).
    /// A `to` older than `from` is a downcast and is refused; a gap no chain of
    /// lenses bridges is refused — both with a typed error, never a silent
    /// identity.
    pub fn upcast(
        &self,
        sector: &str,
        data: &Value,
        from: &Version,
        to: &Version,
    ) -> Result<DerivedView, UpcastError> {
        match to.cmp(from) {
            std::cmp::Ordering::Less => {
                return Err(UpcastError::NotAnUpcast {
                    from: from.clone(),
                    to: to.clone(),
                });
            }
            std::cmp::Ordering::Equal => {
                return Ok(DerivedView {
                    data: data.clone(),
                    derived: true,
                    from: from.to_string(),
                    to: to.to_string(),
                    lens_chain: Vec::new(),
                    lossy: false,
                });
            }
            std::cmp::Ordering::Greater => {}
        }

        let path = self
            .path(sector, from, to)
            .ok_or_else(|| UpcastError::NoPath {
                sector: sector.to_owned(),
                from: from.clone(),
                to: to.clone(),
            })?;

        self.apply(data, from, to, &path)
    }

    /// Upcast `data` for `sector` as far toward `to` as the registered lenses
    /// reach, stopping at the newest reachable version no newer than `to`.
    ///
    /// [`Self::upcast`] demands a path to exactly `to` and refuses anything
    /// short of it. That is right for a caller that asked to see a specific
    /// version, and wrong for a reader that only needs stored data readable at
    /// the current one: a purely additive version bump after a lens leaves no
    /// hop ending at the exact current version, so an exact-path search refuses
    /// a document that the hops it *does* have would have made perfectly
    /// readable. Battery is already in that position — the registry bridges
    /// `1.0.0 → 2.0.0` while the current version is further on, every step
    /// beyond it additive and correctly lens-free.
    ///
    /// The remaining additive gap needs no transform by definition, so the
    /// caller's own deserialize closes it. What this will not do is pretend to
    /// have bridged something: a real gap that no hop touches is refused with
    /// [`UpcastError::NoPath`] rather than returned as a silent identity, and
    /// the returned [`DerivedView`] reports the version actually reached, never
    /// the one requested. `from == to` is the identity, as for
    /// [`Self::upcast`] — there is no gap, so there is no progress to require.
    pub fn upcast_toward(
        &self,
        sector: &str,
        data: &Value,
        from: &Version,
        to: &Version,
    ) -> Result<DerivedView, UpcastError> {
        match to.cmp(from) {
            std::cmp::Ordering::Less => {
                return Err(UpcastError::NotAnUpcast {
                    from: from.clone(),
                    to: to.clone(),
                });
            }
            // No gap to bridge, so no progress to require: the identity, as
            // [`Self::upcast`] gives for the same input.
            std::cmp::Ordering::Equal => return self.apply(data, from, from, &[]),
            std::cmp::Ordering::Greater => {}
        }

        // The newest version reachable that `to` does not precede — fewest hops
        // is already guaranteed per destination by the breadth-first search.
        let (reached, path) = self
            .reachable(sector, from)
            .into_iter()
            .filter(|(v, _)| v <= to)
            .max_by(|(a, _), (b, _)| a.cmp(b))
            .ok_or_else(|| UpcastError::NoPath {
                sector: sector.to_owned(),
                from: from.clone(),
                to: to.clone(),
            })?;

        self.apply(data, from, &reached, &path)
    }

    /// [`Self::upcast_toward`] taking version *strings*, mirroring
    /// [`Self::upcast_str`].
    pub fn upcast_str_toward(
        &self,
        sector: &str,
        data: &Value,
        from: &str,
        to: &str,
    ) -> Result<DerivedView, UpcastError> {
        self.upcast_toward(sector, data, &parse_version(from)?, &parse_version(to)?)
    }

    /// Run `path`'s hops over `data`, recording the chain and whether any hop
    /// was lossy. `reached` is the version the chain actually ends at, which is
    /// not always the one a caller asked for — see [`Self::upcast_toward`].
    fn apply(
        &self,
        data: &Value,
        from: &Version,
        reached: &Version,
        path: &[usize],
    ) -> Result<DerivedView, UpcastError> {
        let mut current = data.clone();
        let mut lens_chain = Vec::new();
        let mut lossy = false;
        for &i in path {
            let lens = &self.lenses[i];
            current = (lens.transform)(&current).map_err(UpcastError::Transform)?;
            lens_chain.push([lens.from.to_string(), lens.to.to_string()]);
            lossy |= lens.lossy;
        }

        Ok(DerivedView {
            data: current,
            derived: true,
            from: from.to_string(),
            to: reached.to_string(),
            lens_chain,
            lossy,
        })
    }

    /// [`Self::upcast`] taking version *strings* — the read-path convenience so
    /// callers (HTTP handlers) don't depend on `semver`. A leading `v` is
    /// tolerated (`v2.0.0`); an unparseable version is a typed refusal.
    pub fn upcast_str(
        &self,
        sector: &str,
        data: &Value,
        from: &str,
        to: &str,
    ) -> Result<DerivedView, UpcastError> {
        self.upcast(sector, data, &parse_version(from)?, &parse_version(to)?)
    }

    /// Fewest-hop lens path (as lens indices) from `from` to `to` for `sector`.
    /// `None` if no path.
    fn path(&self, sector: &str, from: &Version, to: &Version) -> Option<Vec<usize>> {
        self.reachable(sector, from).remove(to)
    }

    /// Every version reachable from `from` for `sector`, each mapped to the
    /// fewest-hop lens path that reaches it, via breadth-first search over the
    /// sector's lens graph. Excludes `from` itself: the identity is not a path.
    fn reachable(&self, sector: &str, from: &Version) -> HashMap<Version, Vec<usize>> {
        let mut queue: VecDeque<Version> = VecDeque::from([from.clone()]);
        let mut paths: HashMap<Version, Vec<usize>> = HashMap::from([(from.clone(), Vec::new())]);

        while let Some(v) = queue.pop_front() {
            // Breadth-first, so the first path found to a version is a shortest
            // one and later arrivals at it are ignored.
            let so_far = paths[&v].clone();
            for (i, lens) in self.lenses.iter().enumerate() {
                if lens.sector == sector && lens.from == v && !paths.contains_key(&lens.to) {
                    let mut path = so_far.clone();
                    path.push(i);
                    paths.insert(lens.to.clone(), path);
                    queue.push_back(lens.to.clone());
                }
            }
        }

        paths.remove(from);
        paths
    }
}

/// Parse a version string, tolerating a leading `v` (`v2.0.0`). An unparseable
/// version is a typed refusal, never a silent identity.
fn parse_version(s: &str) -> Result<Version, UpcastError> {
    s.trim_start_matches('v')
        .parse::<Version>()
        .map_err(|_| UpcastError::BadVersion(s.to_owned()))
}

impl Default for LensRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// The compiled-in lenses shipped with core, versioned alongside the schemas
/// they bridge.
fn builtin_lenses() -> Vec<Lens> {
    vec![
        Lens::new(
            "battery",
            Version::new(1, 0, 0),
            Version::new(2, 0, 0),
            false,
            "EU Battery Regulation 2023/1542 Annex XIII v2.0.0: derives ratedEnergyWh (Wh) \
             from v1 ratedCapacityKwh (kWh); every other v2 field is an optional addition.",
            battery_v1_to_v2,
        ),
        Lens::new(
            "battery",
            Version::new(2, 4, 0),
            Version::new(2, 5, 0),
            false,
            "EU Battery Regulation 2023/1542 Annex VI Part A point 2 (via Annex XIII \
             point 1(a)) v2.5.0: batteryType becomes required and closed. A v2.4.0 \
             record with no batteryType predates the mandate and cannot be upgraded \
             without inventing a value, so this hop refuses rather than default one.",
            battery_v2_4_to_v2_5,
        ),
        Lens::new(
            "battery",
            Version::new(2, 5, 0),
            Version::new(2, 6, 0),
            false,
            "EU Battery Regulation 2023/1542 Annex XIII point 4 v2.6.0: adds the \
             individual-battery tier — dynamicPerformance (4(a)), batteryStatus (4(c)) \
             and usageHistory (4(d)) — all optional, and relaxes \
             expectedLifetimeCycles out of required, since point 1(j) reaches \
             industrial batteries only where lifetime can be expressed in cycles. \
             Also splits Annex XIII point 1(n) into its two figures and point 1(o) \
             into cell and pack resistance; the 1(o) hop refuses rather than guess \
             which measurement a single stored value was.",
            battery_v2_5_to_v2_6,
        ),
        Lens::new(
            "electronics",
            Version::new(1, 1, 0),
            Version::new(1, 2, 0),
            false,
            "Regulation (EU) 2023/1670 Art. 1(1) v1.2.0: productCategory is narrowed to the \
             four device types the regulation actually enumerates (smartphone, other mobile \
             phone, cordless phone, slate tablet). A v1.1.0 record declaring one of the seven \
             removed values has no lawful category to upgrade into, so this hop refuses.",
            electronics_v1_1_to_v1_2,
        ),
        Lens::new(
            "steel",
            Version::new(1, 0, 0),
            Version::new(1, 1, 0),
            false,
            "Cross-sector naming consistency: renames countryOfProduction to \
             countryOfOrigin. Pure rename, no information lost.",
            rename_country_of_production,
        ),
        Lens::new(
            "aluminium",
            Version::new(1, 0, 0),
            Version::new(1, 1, 0),
            false,
            "Cross-sector naming consistency: renames countryOfProduction to \
             countryOfOrigin. Pure rename, no information lost.",
            rename_country_of_production,
        ),
        Lens::new(
            "construction",
            Version::new(1, 0, 0),
            Version::new(1, 1, 0),
            false,
            "Cross-sector naming consistency: renames countryOfManufacture to \
             countryOfOrigin. Pure rename, no information lost.",
            rename_country_of_manufacture,
        ),
        Lens::new(
            "detergent",
            Version::new(1, 0, 0),
            Version::new(1, 1, 0),
            false,
            "Cross-sector naming consistency: renames countryOfManufacture to \
             countryOfOrigin. Pure rename, no information lost.",
            rename_country_of_manufacture,
        ),
        Lens::new(
            "furniture",
            Version::new(1, 0, 0),
            Version::new(1, 1, 0),
            false,
            "Cross-sector naming consistency: renames countryOfManufacture to \
             countryOfOrigin. Pure rename, no information lost.",
            rename_country_of_manufacture,
        ),
        Lens::new(
            "toy",
            Version::new(1, 0, 0),
            Version::new(1, 1, 0),
            false,
            "Cross-sector naming consistency: renames countryOfManufacture to \
             countryOfOrigin. Pure rename, no information lost.",
            rename_country_of_manufacture,
        ),
        Lens::new(
            "textile",
            Version::new(1, 0, 0),
            Version::new(1, 1, 0),
            false,
            "v1.1.0 adds sixteen optional fields (SCIP/SVHC disclosure, per-fibre \
             origin, durability, microplastic shedding) and changes nothing that \
             already existed: the two versions declare identical `required` lists \
             and v1.1.0 removes no property. Purely additive, so the document \
             passes through untouched — but the lens must exist, because a chain \
             cannot cross a gap, and without it a v1.0.0 document has no path to \
             the current version and cannot be read at all.",
            identity,
        ),
        Lens::new(
            "textile",
            Version::new(1, 1, 0),
            Version::new(1, 2, 0),
            false,
            "Cross-sector naming consistency: renames countryOfManufacturing to \
             countryOfOrigin. Pure rename, no information lost.",
            rename_country_of_manufacturing,
        ),
    ]
}

/// A schema step that added only optional fields: the document is already valid
/// under the later version, so it passes through untouched.
///
/// Not redundant. `upcast_str_toward` walks a chain of lenses, and a missing
/// step breaks the whole chain — a document two versions behind cannot reach the
/// current version through a gap, even when the gap itself requires no change.
fn identity(v: &Value) -> Result<Value, LensError> {
    Ok(v.clone())
}

/// Renames a top-level JSON key to `countryOfOrigin`, leaving every other
/// field untouched. Shared by the country-of-origin naming-unification lenses
/// above — each sector's old key differs, so the key name is a parameter.
fn rename_country_field(v: &Value, old_key: &str) -> Result<Value, LensError> {
    let mut out = v.clone();
    let obj = out
        .as_object_mut()
        .ok_or_else(|| LensError("sector data must be a JSON object".to_owned()))?;
    if let Some(val) = obj.remove(old_key) {
        obj.insert("countryOfOrigin".to_owned(), val);
    }
    Ok(out)
}

fn rename_country_of_production(v: &Value) -> Result<Value, LensError> {
    rename_country_field(v, "countryOfProduction")
}

fn rename_country_of_manufacture(v: &Value) -> Result<Value, LensError> {
    rename_country_field(v, "countryOfManufacture")
}

fn rename_country_of_manufacturing(v: &Value) -> Result<Value, LensError> {
    rename_country_field(v, "countryOfManufacturing")
}

/// Battery `v1.0.0 → v2.0.0`: pass all fields through, and derive `ratedEnergyWh`
/// (watt-hours) from `ratedCapacityKwh` (kilowatt-hours) when present. Lossless —
/// v2 is a strict superset whose only computable field from v1 data is the
/// watt-hour restatement of the kilowatt-hour rating.
fn battery_v1_to_v2(v1: &Value) -> Result<Value, LensError> {
    let mut out = v1.clone();
    let obj = out
        .as_object_mut()
        .ok_or_else(|| LensError("battery sector data must be a JSON object".to_owned()))?;
    if let Some(kwh) = obj.get("ratedCapacityKwh").and_then(Value::as_f64)
        && !obj.contains_key("ratedEnergyWh")
    {
        // Restate kWh as Wh, stripping f64 noise (e.g. 100.00000000000001 → 100.0)
        // while keeping up to 6 real decimals.
        let wh = (kwh * 1000.0 * 1_000_000.0).round() / 1_000_000.0;
        obj.insert("ratedEnergyWh".to_owned(), serde_json::json!(wh));
    }
    Ok(out)
}

/// Battery `v2.4.0 → v2.5.0`: passes every field through unchanged. Refuses,
/// rather than defaulting a value, when `batteryType` is absent or not a
/// string — the field becomes required at v2.5.0, and a record written before
/// the mandate existed cannot be made to satisfy it without inventing a
/// category the manufacturer never declared.
fn battery_v2_4_to_v2_5(v: &Value) -> Result<Value, LensError> {
    let obj = v
        .as_object()
        .ok_or_else(|| LensError("battery sector data must be a JSON object".to_owned()))?;
    match obj.get("batteryType") {
        Some(Value::String(_)) => Ok(v.clone()),
        _ => Err(LensError(
            "batteryType is required from v2.5.0 (EU 2023/1542 Annex VI Part A point 2 \
             via Annex XIII point 1(a)); this record predates the mandate and has none, \
             so it cannot be upgraded"
                .to_owned(),
        )),
    }
}

/// Battery `v2.5.0 → v2.6.0`: additions pass through, one key is renamed, and
/// one **refuses**.
///
/// The additive half is straightforward. `dynamicPerformance`, `batteryStatus`
/// and `usageHistory` are optional, and `expectedLifetimeCycles` only *stops*
/// being required, which no existing record can fail. Absence stays absent —
/// materialising an empty point-4 block would assert that the battery reported
/// measurements it never reported.
///
/// **`roundTripEfficiencyPct` and `internalResistanceMohm` are carried
/// verbatim.** Annex XIII splits each into a pair at v2.6.0 — 1(n) into the
/// initial figure and the one at 50 % of cycle-life, 1(o) into cell and pack —
/// but neither legacy value can be assigned to a half of its pair. The first
/// was documented against "50% state of charge", a condition 1(n) does not
/// state; the second cannot say whether it was the cell or the pack. Both keys
/// therefore survive into v2.6.0 under their own names, marked legacy, and the
/// successor fields carry new declarations only. Moving a value would invent
/// the very distinction the split exists to record.
fn battery_v2_5_to_v2_6(v: &Value) -> Result<Value, LensError> {
    if !v.is_object() {
        return Err(LensError(
            "battery sector data must be a JSON object".to_owned(),
        ));
    }
    Ok(v.clone())
}

/// Electronics `v1.1.0 → v1.2.0`: passes every field through unchanged.
/// Refuses, rather than dropping or substituting the record's own category,
/// when `productCategory` is not one of the four device types Regulation
/// (EU) 2023/1670 Art. 1(1) actually enumerates — a record declaring
/// `laptop`, `tv`, or any of the other removed values was written against a
/// category this sector never had a lawful basis for, and there is no value
/// to substitute that would not misdescribe the product.
fn electronics_v1_1_to_v1_2(v: &Value) -> Result<Value, LensError> {
    const VALID: [&str; 4] = [
        "smartphone",
        "other-mobile-phone",
        "cordless-phone",
        "tablet",
    ];
    let obj = v
        .as_object()
        .ok_or_else(|| LensError("electronics sector data must be a JSON object".to_owned()))?;
    match obj.get("productCategory") {
        Some(Value::String(s)) if VALID.contains(&s.as_str()) => Ok(v.clone()),
        _ => Err(LensError(
            "productCategory is not one of the four device types Regulation (EU) 2023/1670 \
             Art. 1(1) enumerates (smartphone, other-mobile-phone, cordless-phone, tablet); \
             this record predates the narrowing and cannot be upgraded"
                .to_owned(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::VersionedSchemaRegistry;

    fn v(s: &str) -> Version {
        s.parse().unwrap()
    }

    /// A minimal but valid v1 battery record (schema-required fields), plus a
    /// rated capacity so the lens has something to derive.
    fn battery_v1() -> Value {
        serde_json::json!({
            "gtin": "09506000134352",
            "batteryChemistry": "LFP",
            "nominalVoltageV": 48.0,
            "nominalCapacityAh": 100.0,
            "expectedLifetimeCycles": 3000,
            "co2ePerUnitKg": 45.2,
            "ratedCapacityKwh": 4.8
        })
    }

    #[test]
    fn battery_v1_upcasts_to_v2_and_validates() {
        let lenses = LensRegistry::new();
        let schemas = VersionedSchemaRegistry::new();
        let original = battery_v1();

        let derived = lenses
            .upcast("battery", &original, &v("1.0.0"), &v("2.0.0"))
            .unwrap();

        // The derived view is honest about its provenance.
        assert!(derived.derived);
        assert!(!derived.lossy);
        assert_eq!(derived.from, "1.0.0");
        assert_eq!(derived.to, "2.0.0");
        assert_eq!(
            derived.lens_chain,
            vec![["1.0.0".to_string(), "2.0.0".to_string()]]
        );

        // The real transform ran: Wh derived from kWh.
        assert_eq!(derived.data["ratedEnergyWh"].as_f64(), Some(4800.0));

        // And the derived data validates against the v2 schema.
        schemas
            .validate("battery", &v("2.0.0"), &derived.data)
            .expect("derived view must validate against v2");

        // The original is untouched (lens clones its input).
        assert!(original.get("ratedEnergyWh").is_none());
    }

    /// A minimal but valid v1.0.0 steel record (schema-required fields).
    fn steel_v1() -> Value {
        serde_json::json!({
            "gtin": "09506000134352",
            "co2ePerTonneSteel": 1.8,
            "recycledScrapContentPct": 35.0,
            "productCategory": "flat",
            "countryOfProduction": "DE",
            "productionRoute": "electric-arc"
        })
    }

    #[test]
    fn steel_v1_upcasts_to_v1_1_and_renames_country_field() {
        let lenses = LensRegistry::new();
        let schemas = VersionedSchemaRegistry::new();
        let original = steel_v1();

        let derived = lenses
            .upcast("steel", &original, &v("1.0.0"), &v("1.1.0"))
            .unwrap();

        assert!(!derived.lossy);
        assert_eq!(derived.data["countryOfOrigin"], "DE");
        assert!(derived.data.get("countryOfProduction").is_none());

        schemas
            .validate("steel", &v("1.1.0"), &derived.data)
            .expect("derived view must validate against v1.1.0");

        // The original is untouched (lens clones its input).
        assert_eq!(original["countryOfProduction"], "DE");
    }

    /// A minimal but valid v1.1.0 textile record (schema-required fields).
    fn textile_v1_1() -> Value {
        serde_json::json!({
            "gtin": "09506000134352",
            "fibreComposition": [{"fibre": "cotton", "pct": 100.0}],
            "countryOfManufacturing": "PT",
            "careInstructions": "Hand wash",
            "chemicalComplianceStandard": "REACH"
        })
    }

    #[test]
    fn textile_v1_1_upcasts_to_v1_2_and_renames_country_field() {
        let lenses = LensRegistry::new();
        let schemas = VersionedSchemaRegistry::new();
        let original = textile_v1_1();

        let derived = lenses
            .upcast("textile", &original, &v("1.1.0"), &v("1.2.0"))
            .unwrap();

        assert!(!derived.lossy);
        assert_eq!(derived.data["countryOfOrigin"], "PT");
        assert!(derived.data.get("countryOfManufacturing").is_none());

        schemas
            .validate("textile", &v("1.2.0"), &derived.data)
            .expect("derived view must validate against v1.2.0");
    }

    #[test]
    fn identity_view_for_same_version_is_lossless() {
        let lenses = LensRegistry::new();
        let data = battery_v1();
        let derived = lenses
            .upcast("battery", &data, &v("1.0.0"), &v("1.0.0"))
            .unwrap();
        assert!(derived.derived);
        assert!(!derived.lossy);
        assert!(derived.lens_chain.is_empty());
        assert_eq!(derived.data, data);
    }

    #[test]
    fn downcast_is_refused() {
        let lenses = LensRegistry::new();
        let err = lenses
            .upcast("battery", &battery_v1(), &v("2.0.0"), &v("1.0.0"))
            .unwrap_err();
        assert!(matches!(err, UpcastError::NotAnUpcast { .. }));
    }

    #[test]
    fn missing_hop_is_a_typed_refusal_not_silent_identity() {
        let lenses = LensRegistry::new();
        // No battery v2 → v3 lens is registered.
        let err = lenses
            .upcast("battery", &battery_v1(), &v("1.0.0"), &v("3.0.0"))
            .unwrap_err();
        assert!(matches!(err, UpcastError::NoPath { .. }));
    }

    // ── Composition + loss propagation, exercised with synthetic lenses ──────

    fn add_a(v: &Value) -> Result<Value, LensError> {
        let mut out = v.clone();
        out.as_object_mut()
            .ok_or_else(|| LensError("not an object".into()))?
            .insert("a".into(), Value::Bool(true));
        Ok(out)
    }

    fn add_b_lossy(v: &Value) -> Result<Value, LensError> {
        let mut out = v.clone();
        let obj = out
            .as_object_mut()
            .ok_or_else(|| LensError("not an object".into()))?;
        obj.insert("b".into(), Value::Bool(true));
        obj.remove("dropped"); // this hop is lossy: it discards a field
        Ok(out)
    }

    #[test]
    fn multi_hop_chain_composes_and_propagates_loss() {
        let reg = LensRegistry::from_lenses(vec![
            Lens::new("demo", v("1.0.0"), v("2.0.0"), false, "add a", add_a),
            Lens::new(
                "demo",
                v("2.0.0"),
                v("3.0.0"),
                true,
                "add b, drop",
                add_b_lossy,
            ),
        ]);
        let data = serde_json::json!({ "dropped": 1 });
        let derived = reg.upcast("demo", &data, &v("1.0.0"), &v("3.0.0")).unwrap();

        assert_eq!(derived.data["a"], Value::Bool(true));
        assert_eq!(derived.data["b"], Value::Bool(true));
        assert!(derived.data.get("dropped").is_none());
        assert!(derived.lossy, "a lossy hop must mark the whole chain lossy");
        assert_eq!(
            derived.lens_chain,
            vec![
                ["1.0.0".to_string(), "2.0.0".to_string()],
                ["2.0.0".to_string(), "3.0.0".to_string()],
            ]
        );
    }

    // ── upcast_toward ───────────────────────────────────────────────────────

    #[test]
    fn toward_reaches_the_newest_registered_version_short_of_the_target() {
        // Battery's registered lens ends at 2.0.0 while the schema has since
        // moved on additively, so no chain lands on the current version and
        // none should have to: `upcast` refuses the gap outright, and a reader
        // that only needs the data readable must not inherit that refusal.
        let reg = LensRegistry::new();
        let current: Version = crate::catalog::SectorCatalog::new()
            .current_schema_version("battery")
            .expect("battery is in the catalog")
            .parse()
            .expect("catalog versions are semver");
        assert!(
            current > v("2.0.0"),
            "this test is only meaningful while battery's current version is \
             past its last lens — got {current}"
        );

        assert!(matches!(
            reg.upcast("battery", &battery_v1(), &v("1.0.0"), &current),
            Err(UpcastError::NoPath { .. })
        ));

        let derived = reg
            .upcast_toward("battery", &battery_v1(), &v("1.0.0"), &current)
            .expect("the 1.0.0 -> 2.0.0 hop must still be applied");

        // It reports the version actually reached, not the one asked for, and
        // the hop really ran.
        assert_eq!(derived.to, "2.0.0");
        assert_eq!(derived.from, "1.0.0");
        assert_eq!(
            derived.lens_chain,
            vec![["1.0.0".to_string(), "2.0.0".to_string()]]
        );
        assert_eq!(derived.data["ratedEnergyWh"].as_f64(), Some(4800.0));
    }

    #[test]
    fn toward_never_overshoots_the_target() {
        let reg = LensRegistry::from_lenses(vec![
            Lens::new("demo", v("1.0.0"), v("2.0.0"), false, "add a", add_a),
            Lens::new("demo", v("2.0.0"), v("3.0.0"), false, "add b", add_b_lossy),
        ]);
        let data = serde_json::json!({ "dropped": 1 });

        // 2.0.0 is reachable and is the ceiling; the 3.0.0 hop must not run.
        let derived = reg
            .upcast_toward("demo", &data, &v("1.0.0"), &v("2.5.0"))
            .expect("2.0.0 is reachable and below the ceiling");
        assert_eq!(derived.to, "2.0.0");
        assert_eq!(derived.data["a"], Value::Bool(true));
        assert!(derived.data.get("b").is_none());
    }

    #[test]
    fn toward_still_refuses_a_gap_no_lens_touches_at_all() {
        // Making progress optional would turn every unbridgeable gap into a
        // silent identity, which is the one thing this module refuses to do.
        //
        // The gap is synthetic on purpose. This asserts a property of the
        // registry, not a fact about any sector's lens coverage — pointing it at
        // a real gap makes it fail the day someone legitimately bridges that gap,
        // which is what happened when textile 1.0.0 → 1.1.0 was added.
        let reg = LensRegistry::from_lenses(vec![Lens::new(
            "demo",
            v("2.0.0"),
            v("3.0.0"),
            false,
            "add b",
            add_b_lossy,
        )]);
        let err = reg
            .upcast_toward("demo", &serde_json::json!({}), &v("1.0.0"), &v("3.0.0"))
            .unwrap_err();
        assert!(
            matches!(err, UpcastError::NoPath { .. }),
            "nothing leaves demo 1.0.0, so this must stay a typed refusal"
        );
    }

    #[test]
    fn toward_refuses_a_downcast() {
        let reg = LensRegistry::new();
        assert!(matches!(
            reg.upcast_toward("battery", &battery_v1(), &v("2.0.0"), &v("1.0.0")),
            Err(UpcastError::NotAnUpcast { .. })
        ));
    }

    #[test]
    fn toward_is_the_identity_for_the_same_version() {
        // No gap means no progress to require — refusing here would make a
        // sector with no lenses at all unreadable at its own current version.
        let reg = LensRegistry::new();
        let data = battery_v1();
        let derived = reg
            .upcast_toward("battery", &data, &v("2.0.0"), &v("2.0.0"))
            .expect("same version is the identity, not a missing path");
        assert!(derived.lens_chain.is_empty());
        assert!(!derived.lossy);
        assert_eq!(derived.to, "2.0.0");
        assert_eq!(derived.data, data);
    }

    #[test]
    fn upcast_str_tolerates_v_prefix_and_refuses_garbage() {
        let reg = LensRegistry::new();
        let data = battery_v1();
        assert!(reg.upcast_str("battery", &data, "v1.0.0", "v2.0.0").is_ok());
        // An unparseable version is a typed refusal, never a silent identity.
        assert!(matches!(
            reg.upcast_str("battery", &data, "1.0.0", "two"),
            Err(UpcastError::BadVersion(_))
        ));
    }

    #[test]
    fn battery_lens_derives_clean_watt_hours() {
        // Correct Wh regardless of f64 noise, and fractional Wh is preserved
        // (not rounded to whole Wh) — distinguishing "strip noise" from "round".
        let reg = LensRegistry::new();
        for (kwh, wh) in [(4.8, 4800.0), (0.1, 100.0), (4.8005, 4800.5)] {
            let mut data = battery_v1();
            data.as_object_mut()
                .unwrap()
                .insert("ratedCapacityKwh".into(), serde_json::json!(kwh));
            let d = reg
                .upcast("battery", &data, &v("1.0.0"), &v("2.0.0"))
                .unwrap();
            assert_eq!(d.data["ratedEnergyWh"].as_f64(), Some(wh), "kwh {kwh}");
        }
    }

    #[test]
    fn battery_v2_4_to_v2_5_passes_through_with_battery_type() {
        let lenses = LensRegistry::new();
        let schemas = VersionedSchemaRegistry::new();
        let mut v2_4 = battery_v1();
        v2_4.as_object_mut()
            .unwrap()
            .insert("batteryType".into(), serde_json::json!("ev"));

        let derived = lenses
            .upcast("battery", &v2_4, &v("2.4.0"), &v("2.5.0"))
            .unwrap();

        assert!(!derived.lossy);
        assert_eq!(derived.data["batteryType"], "ev");
        schemas
            .validate("battery", &v("2.5.0"), &derived.data)
            .expect("derived view must validate against v2.5.0");
    }

    #[test]
    fn battery_v2_4_to_v2_5_refuses_when_battery_type_is_absent() {
        // The one hard question this lens exists to answer: a passport
        // published without batteryType predates the v2.5.0 mandate and
        // cannot be upgraded into satisfying it without inventing a value.
        // A typed refusal is correct here, not a silent identity or a guess.
        let lenses = LensRegistry::new();
        let err = lenses
            .upcast("battery", &battery_v1(), &v("2.4.0"), &v("2.5.0"))
            .unwrap_err();
        assert!(matches!(err, UpcastError::Transform(_)));
    }

    /// A minimal but valid v1.1.0 electronics record (schema-required fields).
    fn electronics_v1_1(product_category: &str) -> Value {
        serde_json::json!({
            "gtin": "09506000134352",
            "productCategory": product_category,
            "energyEfficiencyClass": "B",
            "co2ePerUnitKg": 120.0
        })
    }

    #[test]
    fn electronics_v1_1_to_v1_2_passes_through_a_surviving_category() {
        let lenses = LensRegistry::new();
        let schemas = VersionedSchemaRegistry::new();
        let v1_1 = electronics_v1_1("smartphone");

        let derived = lenses
            .upcast("electronics", &v1_1, &v("1.1.0"), &v("1.2.0"))
            .unwrap();

        assert!(!derived.lossy);
        assert_eq!(derived.data["productCategory"], "smartphone");
        schemas
            .validate("electronics", &v("1.2.0"), &derived.data)
            .expect("derived view must validate against v1.2.0");
    }

    #[test]
    fn electronics_v1_1_to_v1_2_refuses_a_removed_category() {
        // "laptop" was schema-valid at v1.1.0 but has no lawful basis under
        // Regulation (EU) 2023/1670 Art. 1(1) — there is no v1.2.0 value to
        // substitute that would not misdescribe the product.
        let lenses = LensRegistry::new();
        let err = lenses
            .upcast(
                "electronics",
                &electronics_v1_1("laptop"),
                &v("1.1.0"),
                &v("1.2.0"),
            )
            .unwrap_err();
        assert!(matches!(err, UpcastError::Transform(_)));
    }

    #[test]
    fn battery_v2_5_to_v2_6_upgrades_a_record_with_no_cycle_count() {
        // The v2.6.0 relaxation exists for industrial batteries whose lifetime
        // cannot be expressed in cycles (Annex XIII point 1(j)). A record with
        // no expectedLifetimeCycles fails v2.5.0 and must pass v2.6.0 — this is
        // the opposite of the v2.4.0 hop above, which refuses. A relaxation
        // cannot strand a document; a new obligation can.
        let lenses = LensRegistry::new();
        let schemas = VersionedSchemaRegistry::new();
        let mut data = battery_v1();
        let obj = data.as_object_mut().unwrap();
        obj.insert("batteryType".into(), serde_json::json!("industrial"));
        obj.remove("expectedLifetimeCycles");

        assert!(
            schemas.validate("battery", &v("2.5.0"), &data).is_err(),
            "the fixture must be one v2.5.0 rejects, or this proves nothing"
        );

        let derived = lenses
            .upcast("battery", &data, &v("2.5.0"), &v("2.6.0"))
            .unwrap();
        assert!(!derived.lossy);
        schemas
            .validate("battery", &v("2.6.0"), &derived.data)
            .expect("derived view must validate against v2.6.0");
    }

    #[test]
    fn battery_v2_5_to_v2_6_does_not_invent_an_empty_point_4_block() {
        // Absence stays absent. Materialising an empty dynamicPerformance would
        // assert that the battery reported measured values it never reported.
        let lenses = LensRegistry::new();
        let mut data = battery_v1();
        data.as_object_mut()
            .unwrap()
            .insert("batteryType".into(), serde_json::json!("ev"));

        let derived = lenses
            .upcast("battery", &data, &v("2.5.0"), &v("2.6.0"))
            .unwrap();

        for key in ["dynamicPerformance", "batteryStatus", "usageHistory"] {
            assert!(
                derived.data.get(key).is_none(),
                "the lens materialised '{key}', which the source never carried"
            );
        }
    }

    #[test]
    fn battery_v2_5_to_v2_6_carries_both_legacy_keys_verbatim() {
        // Neither legacy value can be assigned to a half of its Annex XIII pair
        // — one names a condition the annex does not state, the other cannot
        // say whether it was cell or pack. So both survive under their own
        // names rather than being moved, guessed at, or refused.
        let lenses = LensRegistry::new();
        let schemas = VersionedSchemaRegistry::new();
        let mut data = battery_v1();
        let obj = data.as_object_mut().unwrap();
        obj.insert("batteryType".into(), serde_json::json!("ev"));
        obj.insert("roundTripEfficiencyPct".into(), serde_json::json!(91.5));
        obj.insert("internalResistanceMohm".into(), serde_json::json!(12.0));

        let derived = lenses
            .upcast("battery", &data, &v("2.5.0"), &v("2.6.0"))
            .unwrap();

        assert!(!derived.lossy);
        assert_eq!(derived.data["roundTripEfficiencyPct"], 91.5);
        assert_eq!(derived.data["internalResistanceMohm"], 12.0);
        for successor in [
            "roundTripEfficiencyAtHalfCycleLifePct",
            "initialRoundTripEfficiencyPct",
            "internalCellResistanceMohm",
            "internalPackResistanceMohm",
        ] {
            assert!(
                derived.data.get(successor).is_none(),
                "the lens populated '{successor}', inventing a distinction the                  source never made"
            );
        }
        schemas
            .validate("battery", &v("2.6.0"), &derived.data)
            .expect("both legacy keys validate against v2.6.0");
    }

    #[test]
    fn battery_v2_6_accepts_the_individual_battery_tier() {
        let schemas = VersionedSchemaRegistry::new();
        let mut data = battery_v1();
        let obj = data.as_object_mut().unwrap();
        obj.insert("batteryType".into(), serde_json::json!("ev"));
        obj.insert(
            "dynamicPerformance".into(),
            serde_json::json!({ "ratedCapacityAh": 92.0, "capacityFadePct": 8.0 }),
        );
        obj.insert("batteryStatus".into(), serde_json::json!("repurposed"));
        obj.insert(
            "usageHistory".into(),
            serde_json::json!({
                "chargeDischargeCycles": 412,
                "stateOfCharge": [
                    { "recordedAt": "2026-08-11T09:00:00Z", "stateOfChargePct": 61.5 }
                ]
            }),
        );

        schemas
            .validate("battery", &v("2.6.0"), &data)
            .expect("the point 4 tier validates");
    }

    #[test]
    fn battery_v2_6_refuses_a_status_the_annex_does_not_enumerate() {
        // Annex XIII point 4(c) spells the set out inline, so it is closed for
        // the same reason batteryType is.
        let schemas = VersionedSchemaRegistry::new();
        let mut data = battery_v1();
        let obj = data.as_object_mut().unwrap();
        obj.insert("batteryType".into(), serde_json::json!("ev"));
        obj.insert("batteryStatus".into(), serde_json::json!("refurbished"));

        assert!(
            schemas.validate("battery", &v("2.6.0"), &data).is_err(),
            "'refurbished' is not one of the five the annex names"
        );
    }

    #[test]
    fn battery_lens_with_nothing_to_derive_still_validates_against_v2() {
        // A v1 record with no ratedCapacityKwh: the lens has nothing to derive,
        // and the result must still validate against v2 (all v2 additions optional).
        let mut data = battery_v1();
        data.as_object_mut().unwrap().remove("ratedCapacityKwh");
        let reg = LensRegistry::new();
        let schemas = VersionedSchemaRegistry::new();
        let derived = reg
            .upcast("battery", &data, &v("1.0.0"), &v("2.0.0"))
            .unwrap();
        assert!(derived.data.get("ratedEnergyWh").is_none());
        schemas
            .validate("battery", &v("2.0.0"), &derived.data)
            .expect("a v1 record with no rated capacity still validates against v2");
    }
}
