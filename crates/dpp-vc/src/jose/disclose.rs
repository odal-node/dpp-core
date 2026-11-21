//! Concealing the claims a disclosure path names, when a credential or presentation is
//! secured as an SD-JWT.
//!
//! A path is a [`ClaimPath`]: object members by name and array elements by position,
//! counted in the document as the issuer wrote it. Each claim a path ends at becomes a
//! Disclosure (RFC 9901 clauses 4.2.1 and 4.2.2) built by [`conceal`] or
//! [`conceal_elements`], so the digests and the placeholders are the ones the SD-JWT
//! primitive makes. A path that goes deeper than another does is concealed first, so a
//! claim hidden inside a hidden claim sits in the outer Disclosure's own value
//! (clause 4.2.6).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use dpp_crypto::sd_jwt::{ClaimPath, Disclosure, PathSegment, conceal, conceal_elements};

use super::error::JoseError;

/// The names SD-JWT reserves (RFC 9901 clauses 4.1.1 and 4.2.1). A document carrying one
/// would be read back as something else, so it is not signed.
const RESERVED: [&str; 3] = ["_sd", "_sd_alg", "..."];

/// A path, with how many of its segments are already spent.
#[derive(Clone, Copy)]
struct Pending<'a> {
    path: &'a ClaimPath,
    at: usize,
}

impl<'a> Pending<'a> {
    fn segment(self) -> &'a PathSegment {
        &self.path.segments()[self.at]
    }

    /// Whether the segment in hand is the last, so the claim it reaches is concealed.
    fn ends_here(self) -> bool {
        self.at + 1 == self.path.segments().len()
    }

    fn next(self) -> Self {
        Self {
            at: self.at + 1,
            ..self
        }
    }

    fn not_found(self) -> JoseError {
        JoseError::DisclosurePathNotFound {
            path: self.path.to_string(),
        }
    }
}

/// `document` with the claims `paths` name replaced by digests, and the Disclosures that
/// reopen it. The `_sd_alg` claim is not added here.
///
/// # Errors
///
/// [`JoseError::ReservedSdJwtMember`] when the document has a member `_sd`, `_sd_alg` or
/// `...`, and [`JoseError::DisclosurePathNotFound`] for a path that selects nothing.
/// `null` in a path stands for every element of an array, and every one of them must hold
/// the rest of the path.
pub(crate) fn conceal_paths(
    document: &Map<String, Value>,
    paths: &[ClaimPath],
) -> Result<(Map<String, Value>, Vec<Disclosure>), JoseError> {
    check_no_reserved_member(&Value::Object(document.clone()))?;
    let pending: Vec<Pending<'_>> = paths.iter().map(|path| Pending { path, at: 0 }).collect();
    let mut disclosures = Vec::new();
    let concealed = conceal_object(document, &pending, &mut disclosures)?;
    Ok((concealed, disclosures))
}

fn check_no_reserved_member(value: &Value) -> Result<(), JoseError> {
    match value {
        Value::Object(map) => {
            for name in RESERVED {
                if map.contains_key(name) {
                    return Err(JoseError::ReservedSdJwtMember { name });
                }
            }
            map.values().try_for_each(check_no_reserved_member)
        }
        Value::Array(items) => items.iter().try_for_each(check_no_reserved_member),
        _ => Ok(()),
    }
}

fn conceal_value(
    value: &Value,
    pending: &[Pending<'_>],
    disclosures: &mut Vec<Disclosure>,
) -> Result<Value, JoseError> {
    match value {
        Value::Object(map) => conceal_object(map, pending, disclosures).map(Value::Object),
        Value::Array(items) => conceal_array(items, pending, disclosures).map(Value::Array),
        _ => Err(pending[0].not_found()),
    }
}

fn conceal_object(
    object: &Map<String, Value>,
    pending: &[Pending<'_>],
    disclosures: &mut Vec<Disclosure>,
) -> Result<Map<String, Value>, JoseError> {
    let mut hide: BTreeSet<&str> = BTreeSet::new();
    let mut deeper: BTreeMap<&str, Vec<Pending<'_>>> = BTreeMap::new();
    for pending in pending {
        let PathSegment::Key(name) = pending.segment() else {
            return Err(pending.not_found());
        };
        if !object.contains_key(name) {
            return Err(pending.not_found());
        }
        if pending.ends_here() {
            hide.insert(name);
        } else {
            deeper.entry(name).or_default().push(pending.next());
        }
    }

    let mut rewritten = object.clone();
    for (name, below) in deeper {
        let value = conceal_value(&object[name], &below, disclosures)?;
        rewritten.insert(name.to_owned(), value);
    }
    let (kept, mut produced) =
        conceal(&rewritten, |name| hide.contains(name)).map_err(|e| JoseError::Signing {
            reason: e.to_string(),
        })?;
    disclosures.append(&mut produced);
    Ok(kept)
}

fn conceal_array(
    items: &[Value],
    pending: &[Pending<'_>],
    disclosures: &mut Vec<Disclosure>,
) -> Result<Vec<Value>, JoseError> {
    let mut hide: BTreeSet<usize> = BTreeSet::new();
    let mut deeper: BTreeMap<usize, Vec<Pending<'_>>> = BTreeMap::new();
    for pending in pending {
        let positions: Vec<usize> = match pending.segment() {
            PathSegment::Index(position) if *position < items.len() => vec![*position],
            PathSegment::AllElements if !items.is_empty() => (0..items.len()).collect(),
            _ => return Err(pending.not_found()),
        };
        for position in positions {
            if pending.ends_here() {
                hide.insert(position);
            } else {
                deeper.entry(position).or_default().push(pending.next());
            }
        }
    }

    let mut rewritten = items.to_vec();
    for (position, below) in deeper {
        rewritten[position] = conceal_value(&items[position], &below, disclosures)?;
    }
    let (kept, mut produced) = conceal_elements(&rewritten, |position, _| hide.contains(&position));
    disclosures.append(&mut produced);
    Ok(kept)
}
