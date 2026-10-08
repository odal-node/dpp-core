//! Issuing a claim tree to a plan, and working out what any presentation of it
//! must reveal.
//!
//! A plan is a set of paths to hide. Everything else follows from it. The
//! expectation for a presentation is computed here from the original claims and
//! the set of paths presented, by a walk that shares nothing with
//! [`SdJwt::disclosed_payload`]: the corpus is only worth judging if the answer
//! it carries did not come from the code being judged.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::sd_jwt::{Disclosure, build_payload, conceal, conceal_elements};

/// One step down a claim tree.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Seg {
    Key(String),
    Idx(usize),
}

pub(super) type Path = Vec<Seg>;

/// A path as a test would say it: `&["address", "region"]`, and `#` marks an
/// array index, so `&["nationalities", "#1"]` is the second nationality.
pub(super) fn path(spec: &[&str]) -> Path {
    spec.iter()
        .map(|s| match s.strip_prefix('#') {
            Some(n) => Seg::Idx(n.parse().expect("an index after #")),
            None => Seg::Key((*s).to_owned()),
        })
        .collect()
}

pub(super) fn paths(specs: &[&[&str]]) -> BTreeSet<Path> {
    specs.iter().map(|s| path(s)).collect()
}

/// A claim tree issued to a plan.
pub(super) struct Issued {
    /// The JWT payload: digests and placeholders where claims were hidden, and
    /// `_sd_alg` when anything was.
    pub(super) payload: Value,
    pub(super) disclosures: Vec<Disclosure>,
    /// The digest of the Disclosure that hides each path.
    pub(super) digest_at: BTreeMap<Path, String>,
}

/// Issue `claims` so that exactly the paths in `hidden` are concealed.
///
/// Children are processed before their parent, which is what makes a concealed
/// object carry its own concealed members inside its Disclosure: RFC 9901
/// section 6.3, the recursive structure.
pub(super) fn issue_to_plan(claims: &Value, hidden: &BTreeSet<Path>) -> Issued {
    let mut found: Vec<(Path, Disclosure)> = Vec::new();
    let mut at = Path::new();
    let top = match walk(claims, &mut at, hidden, &mut found) {
        Value::Object(map) => map,
        other => panic!("claims must be an object, got {other}"),
    };
    let any = !found.is_empty();
    let payload = build_payload(top, any);
    let digest_at = found.iter().map(|(p, d)| (p.clone(), d.digest())).collect();
    Issued {
        payload,
        disclosures: found.into_iter().map(|(_, d)| d).collect(),
        digest_at,
    }
}

fn child(at: &Path, seg: Seg) -> Path {
    let mut p = at.clone();
    p.push(seg);
    p
}

fn walk(
    value: &Value,
    at: &mut Path,
    hidden: &BTreeSet<Path>,
    found: &mut Vec<(Path, Disclosure)>,
) -> Value {
    match value {
        Value::Object(map) => {
            let mut processed = Map::new();
            for (name, member) in map {
                at.push(Seg::Key(name.clone()));
                let done = walk(member, at, hidden, found);
                at.pop();
                processed.insert(name.clone(), done);
            }
            let (kept, disclosures) = conceal(&processed, |name| {
                hidden.contains(&child(at, Seg::Key(name.to_owned())))
            })
            .expect("a plan never hides a reserved claim name");
            for disclosure in disclosures {
                let name = disclosure.claim_name().expect("a property").to_owned();
                found.push((child(at, Seg::Key(name)), disclosure));
            }
            Value::Object(kept)
        }
        Value::Array(items) => {
            let mut processed = Vec::with_capacity(items.len());
            for (i, item) in items.iter().enumerate() {
                at.push(Seg::Idx(i));
                processed.push(walk(item, at, hidden, found));
                at.pop();
            }
            let positions: Vec<usize> = (0..processed.len())
                .filter(|i| hidden.contains(&child(at, Seg::Idx(*i))))
                .collect();
            let (kept, disclosures) = conceal_elements(&processed, |i, _| positions.contains(&i));
            for (disclosure, i) in disclosures.into_iter().zip(positions) {
                found.push((child(at, Seg::Idx(i)), disclosure));
            }
            Value::Array(kept)
        }
        other => other.clone(),
    }
}

/// What a verifier must see when only `presented` of the hidden paths are
/// presented: the claims, minus every hidden path not presented.
///
/// An array loses the elements that are not presented, and the ones after them
/// move up, because that is what RFC 9901 clause 7.1 step 3.d does with a
/// placeholder nobody holds a Disclosure for.
pub(super) fn reveal(claims: &Value, hidden: &BTreeSet<Path>, presented: &BTreeSet<Path>) -> Value {
    fn go(
        value: &Value,
        at: &mut Path,
        hidden: &BTreeSet<Path>,
        presented: &BTreeSet<Path>,
    ) -> Value {
        let shown = |p: &Path| !hidden.contains(p) || presented.contains(p);
        match value {
            Value::Object(map) => {
                let mut out = Map::new();
                for (name, member) in map {
                    at.push(Seg::Key(name.clone()));
                    if shown(at) {
                        out.insert(name.clone(), go(member, at, hidden, presented));
                    }
                    at.pop();
                }
                Value::Object(out)
            }
            Value::Array(items) => {
                let mut out = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    at.push(Seg::Idx(i));
                    if shown(at) {
                        out.push(go(item, at, hidden, presented));
                    }
                    at.pop();
                }
                Value::Array(out)
            }
            other => other.clone(),
        }
    }
    go(claims, &mut Path::new(), hidden, presented)
}

/// Whether a holder can present exactly `presented`: a Disclosure for something
/// inside a hidden object is useless unless the object's own is presented too,
/// and a token that carried it anyway would carry an unreferenced Disclosure.
pub(super) fn reachable(hidden: &BTreeSet<Path>, presented: &BTreeSet<Path>) -> bool {
    presented.iter().all(|p| {
        (1..p.len()).all(|n| {
            let ancestor = &p[..n];
            !hidden.contains(ancestor) || presented.contains(ancestor)
        })
    })
}

/// Every reachable set of presented paths, for a plan small enough to list.
pub(super) fn every_presentation(hidden: &BTreeSet<Path>) -> Vec<BTreeSet<Path>> {
    assert!(hidden.len() <= 12, "too many hidden paths to enumerate");
    let all: Vec<&Path> = hidden.iter().collect();
    (0u32..(1 << all.len()))
        .map(|mask| {
            all.iter()
                .enumerate()
                .filter(|(i, _)| mask & (1 << i) != 0)
                .map(|(_, p)| (*p).clone())
                .collect::<BTreeSet<_>>()
        })
        .filter(|set| reachable(hidden, set))
        .collect()
}

/// A deterministic generator: SplitMix64.
///
/// Written out because the corpus has to be made of the same cases on every
/// machine and in every run, so a failure in CI can be reproduced from the case
/// id alone. The bytes differ between runs, since salts are random.
pub(super) struct Rng(pub(super) u64);

impl Rng {
    pub(super) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..n`.
    pub(super) fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    pub(super) fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }
}

/// Distinct reachable presentations drawn at random, for a plan too large to
/// list: nothing, everything, and up to `count` others.
pub(super) fn sampled_presentations(
    hidden: &BTreeSet<Path>,
    rng: &mut Rng,
    count: usize,
) -> Vec<BTreeSet<Path>> {
    let mut seen: BTreeSet<BTreeSet<Path>> = BTreeSet::new();
    seen.insert(BTreeSet::new());
    seen.insert(hidden.clone());
    // Bounded, because a plan with few reachable sets would never fill `count`.
    for _ in 0..count * 20 {
        if seen.len() >= count + 2 {
            break;
        }
        let set: BTreeSet<Path> = hidden.iter().filter(|_| rng.chance(50)).cloned().collect();
        if reachable(hidden, &set) {
            seen.insert(set);
        }
    }
    seen.into_iter().collect()
}
