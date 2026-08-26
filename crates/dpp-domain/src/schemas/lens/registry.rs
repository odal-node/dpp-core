//! [`LensRegistry`] — composes single-hop lenses into multi-hop chains at read time.

use std::collections::{HashMap, VecDeque};

use semver::Version;
use serde_json::Value;

use super::builtin::builtin_lenses;
use super::derived_view::DerivedView;
use super::transform::Lens;
use super::upcast_error::UpcastError;

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

    /// Upcast `data` for `product_group` from `from` up to `to`, composing single-hop
    /// lenses along the fewest-hop path.
    ///
    /// `from == to` is the identity (a no-loss derived view of the same version).
    /// A `to` older than `from` is a downcast and is refused; a gap no chain of
    /// lenses bridges is refused — both with a typed error, never a silent
    /// identity.
    pub fn upcast(
        &self,
        product_group: &str,
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
            .path(product_group, from, to)
            .ok_or_else(|| UpcastError::NoPath {
                product_group: product_group.to_owned(),
                from: from.clone(),
                to: to.clone(),
            })?;

        self.apply(data, from, to, &path)
    }

    /// Upcast `data` for `product_group` as far toward `to` as the registered lenses
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
        product_group: &str,
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
            .reachable(product_group, from)
            .into_iter()
            .filter(|(v, _)| v <= to)
            .max_by(|(a, _), (b, _)| a.cmp(b))
            .ok_or_else(|| UpcastError::NoPath {
                product_group: product_group.to_owned(),
                from: from.clone(),
                to: to.clone(),
            })?;

        self.apply(data, from, &reached, &path)
    }

    /// [`Self::upcast_toward`] taking version *strings*, mirroring
    /// [`Self::upcast_str`].
    pub fn upcast_str_toward(
        &self,
        product_group: &str,
        data: &Value,
        from: &str,
        to: &str,
    ) -> Result<DerivedView, UpcastError> {
        self.upcast_toward(
            product_group,
            data,
            &parse_version(from)?,
            &parse_version(to)?,
        )
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
        product_group: &str,
        data: &Value,
        from: &str,
        to: &str,
    ) -> Result<DerivedView, UpcastError> {
        self.upcast(
            product_group,
            data,
            &parse_version(from)?,
            &parse_version(to)?,
        )
    }

    /// Fewest-hop lens path (as lens indices) from `from` to `to` for `product_group`.
    /// `None` if no path.
    fn path(&self, product_group: &str, from: &Version, to: &Version) -> Option<Vec<usize>> {
        self.reachable(product_group, from).remove(to)
    }

    /// Every version reachable from `from` for `product_group`, each mapped to the
    /// fewest-hop lens path that reaches it, via breadth-first search over the
    /// product group's lens graph. Excludes `from` itself: the identity is not a path.
    fn reachable(&self, product_group: &str, from: &Version) -> HashMap<Version, Vec<usize>> {
        let mut queue: VecDeque<Version> = VecDeque::from([from.clone()]);
        let mut paths: HashMap<Version, Vec<usize>> = HashMap::from([(from.clone(), Vec::new())]);

        while let Some(v) = queue.pop_front() {
            // Breadth-first, so the first path found to a version is a shortest
            // one and later arrivals at it are ignored.
            let so_far = paths[&v].clone();
            for (i, lens) in self.lenses.iter().enumerate() {
                if lens.product_group == product_group
                    && lens.from == v
                    && !paths.contains_key(&lens.to)
                {
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
