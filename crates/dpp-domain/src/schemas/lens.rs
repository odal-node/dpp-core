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

use std::collections::{HashMap, HashSet, VecDeque};

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

        let mut current = data.clone();
        let mut lens_chain = Vec::new();
        let mut lossy = false;
        for &i in &path {
            let lens = &self.lenses[i];
            current = (lens.transform)(&current).map_err(UpcastError::Transform)?;
            lens_chain.push([lens.from.to_string(), lens.to.to_string()]);
            lossy |= lens.lossy;
        }

        Ok(DerivedView {
            data: current,
            derived: true,
            from: from.to_string(),
            to: to.to_string(),
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
        let parse = |s: &str| {
            s.trim_start_matches('v')
                .parse::<Version>()
                .map_err(|_| UpcastError::BadVersion(s.to_owned()))
        };
        self.upcast(sector, data, &parse(from)?, &parse(to)?)
    }

    /// Fewest-hop lens path (as lens indices) from `from` to `to` for `sector`,
    /// via breadth-first search over the sector's lens graph. `None` if no path.
    fn path(&self, sector: &str, from: &Version, to: &Version) -> Option<Vec<usize>> {
        let mut queue: VecDeque<Version> = VecDeque::from([from.clone()]);
        let mut visited: HashSet<Version> = HashSet::from([from.clone()]);
        // Reached-version → index of the lens that reached it.
        let mut prev: HashMap<Version, usize> = HashMap::new();

        while let Some(v) = queue.pop_front() {
            if &v == to {
                break;
            }
            for (i, lens) in self.lenses.iter().enumerate() {
                if lens.sector == sector && lens.from == v && visited.insert(lens.to.clone()) {
                    prev.insert(lens.to.clone(), i);
                    queue.push_back(lens.to.clone());
                }
            }
        }

        if !prev.contains_key(to) {
            return None;
        }
        let mut path = Vec::new();
        let mut cur = to.clone();
        while &cur != from {
            let i = *prev.get(&cur)?;
            path.push(i);
            cur = self.lenses[i].from.clone();
        }
        path.reverse();
        Some(path)
    }
}

impl Default for LensRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// The compiled-in lenses shipped with core, versioned alongside the schemas
/// they bridge.
fn builtin_lenses() -> Vec<Lens> {
    vec![Lens::new(
        "battery",
        Version::new(1, 0, 0),
        Version::new(2, 0, 0),
        false,
        "EU Battery Regulation 2023/1542 Annex XIII v2.0.0: derives ratedEnergyWh (Wh) \
         from v1 ratedCapacityKwh (kWh); every other v2 field is an optional addition.",
        battery_v1_to_v2,
    )]
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
