//! Choosing what to present by claim path.
//!
//! A Holder presents part of an SD-JWT by choosing Disclosures, and
//! [`SdJwt::present`] takes their digests. Getting from the claims a Holder wants
//! to those digests used to be the caller's: [`SdJwt::digests_for_claim`] matches a
//! claim *name* at any depth, which cannot name a nested claim without the claims
//! above it, and cannot name an array element at all, since an element has no
//! name. [`SdJwt::digests_for_paths`] takes the claims wanted as paths, and returns
//! the Disclosures that reveal them and no others.
//!
//! # The path
//!
//! The syntax of draft-ietf-oauth-sd-jwt-vc-19 clause 5.6.1, which is also the
//! claims path of OpenID for Verifiable Presentations: a sequence of an object
//! member's name, a non-negative integer for one array element, and `null` for
//! every element. No conformance to the draft is claimed.
//!
//! # What a path reveals
//!
//! Exactly the claim it names, plus the Disclosures that stand between the claim and
//! the Issuer-signed payload. A path to `address.region` returns the Disclosure for
//! `region` and the one for `address`, whose own value holds the digest of `region`.
//! A path to `address` returns `address`'s Disclosure alone: its members that are
//! hidden stay hidden, and each is chosen by a path of its own. A claim that is in
//! the clear needs no Disclosure, and naming it is not an error.
//!
//! # Where an array element is
//!
//! At its position in the array as the Issuer made it. A hidden element counts, and
//! so does one in the clear; the draft's text is that a path is read as if every
//! selectively disclosable claim were disclosed. So with `[a, b, c, d]` and `a` and
//! `c` hidden, `2` is `c`. It is not the second element a Verifier would see, which
//! is `d`. A digest placeholder with no Disclosure behind it does not count, because
//! it is a decoy (clause 4.2.5), and a decoy is not an element of the array.
//!
//! 🚨 **That is exact only for a token that holds every Disclosure the Issuer
//! made**, which is what an Issuer hands a Holder. A token that was already
//! presented in part cannot be told from one with decoys: an element whose
//! Disclosure is missing is skipped like a decoy, and every position after it moves
//! down by one. A caller selecting from such a token by position is naming a
//! different element than the Issuer's array has at that number. Selecting by name
//! is not affected.
//!
//! # It fails closed
//!
//! A path that selects nothing is [`SdJwtError::ClaimNotInToken`], naming the path,
//! and nothing is returned for the others. The libraries this was read against
//! differ: one returns nothing for the whole request, one ignores the path and
//! presents the rest. Ignoring is the failure to avoid here, since a Holder who
//! asked for a claim and silently did not present it has not said so to anyone.
//!
//! A wildcard that matches some elements and not others is not an error.
//! `["degrees", null, "type"]` selects the `type` of each degree that has one.
//!
//! # It checks its own work
//!
//! The Disclosures are chosen by walking the signed payload. They are then checked
//! three ways before they are returned. Reading the result the way a Verifier will
//! must succeed (clause 7.2 step 2 and clause 7.1 step 5), so no Disclosure is left
//! with nothing referring to it. Each path must select the same number of claims,
//! and the same value for each scalar, as plain navigation of the fully processed
//! payload does, which is the draft's "as if all were disclosed" read a second way.
//! And each path must still find its claim in the presentation. A failure of any
//! is [`SdJwtError::SelectionDisagrees`]. It means this module is wrong and not
//! that the input is.

use std::collections::{HashMap, HashSet};
use std::fmt;

use serde_json::Value;

use super::builder::{
    ARRAY_ELEMENT_CLAIM, SD_ALG_CLAIM, SD_CLAIM, SdJwt, decode_jwt_payload, placeholder_digest,
};
use super::disclosure::Disclosure;
use super::error::SdJwtError;

/// One step of a [`ClaimPath`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    /// An object member, by name.
    Key(String),
    /// One array element, by its position in the array as the Issuer made it.
    Index(usize),
    /// Every element of an array. `null` in the draft's JSON form.
    AllElements,
}

/// The way to a claim: a sequence of [`PathSegment`]s from the root of the payload.
///
/// ```
/// use dpp_crypto::sd_jwt::ClaimPath;
///
/// let region = ClaimPath::claim("address").key("region");
/// let second_nationality = ClaimPath::claim("nationalities").index(1);
/// assert_eq!(region.to_string(), r#"["address","region"]"#);
/// assert_eq!(second_nationality.to_string(), r#"["nationalities",1]"#);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimPath(Vec<PathSegment>);

impl ClaimPath {
    /// A path of the given segments. At least one is required.
    ///
    /// # Errors
    ///
    /// [`SdJwtError::InvalidClaimPath`] for no segments, which names no claim.
    pub fn new(segments: Vec<PathSegment>) -> Result<Self, SdJwtError> {
        if segments.is_empty() {
            return Err(SdJwtError::InvalidClaimPath);
        }
        Ok(Self(segments))
    }

    /// A path of one segment: the top-level claim `name`.
    pub fn claim(name: impl Into<String>) -> Self {
        Self(vec![PathSegment::Key(name.into())])
    }

    /// This path, continued into the object member `name`.
    #[must_use]
    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.0.push(PathSegment::Key(name.into()));
        self
    }

    /// This path, continued into the array element at `position`.
    #[must_use]
    pub fn index(mut self, position: usize) -> Self {
        self.0.push(PathSegment::Index(position));
        self
    }

    /// This path, continued into every element of the array.
    #[must_use]
    pub fn all_elements(mut self) -> Self {
        self.0.push(PathSegment::AllElements);
        self
    }

    /// The segments, outermost first.
    pub fn segments(&self) -> &[PathSegment] {
        &self.0
    }

    /// A path in the draft's JSON form: a non-empty array of strings, non-negative
    /// integers and `null`, as in `["degrees", null, "type"]`.
    ///
    /// # Errors
    ///
    /// [`SdJwtError::InvalidClaimPath`] for anything else, including an empty
    /// array, a negative or fractional number, a boolean, and an object.
    pub fn from_json(value: &Value) -> Result<Self, SdJwtError> {
        let segments = value
            .as_array()
            .ok_or(SdJwtError::InvalidClaimPath)?
            .iter()
            .map(|segment| match segment {
                Value::String(name) => Ok(PathSegment::Key(name.clone())),
                Value::Null => Ok(PathSegment::AllElements),
                Value::Number(n) => n
                    .as_u64()
                    .and_then(|i| usize::try_from(i).ok())
                    .map(PathSegment::Index)
                    .ok_or(SdJwtError::InvalidClaimPath),
                _ => Err(SdJwtError::InvalidClaimPath),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(segments)
    }
}

/// The path in its JSON form, for error messages.
impl fmt::Display for ClaimPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let segments: Vec<Value> = self
            .0
            .iter()
            .map(|segment| match segment {
                PathSegment::Key(name) => Value::String(name.clone()),
                PathSegment::Index(i) => Value::from(*i),
                PathSegment::AllElements => Value::Null,
            })
            .collect();
        write!(f, "{}", Value::Array(segments))
    }
}

/// A claim a path reached: its value as the token holds it, and the digests of the
/// Disclosures that had to be opened to get there.
struct Hit<'a> {
    value: &'a Value,
    needed: Vec<String>,
}

/// The names that are the mechanism's and never a claim: `_sd` and `_sd_alg` are
/// removed by processing, and `...` marks an array element.
fn is_mechanism(name: &str) -> bool {
    matches!(name, SD_CLAIM | SD_ALG_CLAIM | ARRAY_ELEMENT_CLAIM)
}

/// Walk `node` along `rest`, through the token as the Issuer signed it and the
/// Disclosures `held`, and record every claim the path reaches.
fn walk<'a>(
    node: &'a Value,
    rest: &[PathSegment],
    needed: &mut Vec<String>,
    held: &HashMap<String, &'a Disclosure>,
    hits: &mut Vec<Hit<'a>>,
) {
    let Some((segment, rest)) = rest.split_first() else {
        hits.push(Hit {
            value: node,
            needed: needed.clone(),
        });
        return;
    };
    match (node, segment) {
        (Value::Object(map), PathSegment::Key(name)) if !is_mechanism(name) => {
            if let Some(child) = map.get(name) {
                walk(child, rest, needed, held, hits);
                return;
            }
            // Hidden: the member is in a Disclosure named by one of this object's
            // digests. A digest this token holds nothing for is a decoy, or a
            // claim that was not given, and names nothing.
            let digests = map.get(SD_CLAIM).and_then(Value::as_array);
            for digest in digests.into_iter().flatten().filter_map(Value::as_str) {
                if let Some(disclosure) = held.get(digest)
                    && disclosure.claim_name() == Some(name.as_str())
                {
                    needed.push(digest.to_owned());
                    walk(disclosure.claim_value(), rest, needed, held, hits);
                    needed.pop();
                }
            }
        }
        (Value::Array(items), PathSegment::Index(_) | PathSegment::AllElements) => {
            let mut position = 0usize;
            for item in items {
                let (value, digest) = match placeholder_digest(item) {
                    None => (item, None),
                    Some(digest) => match held.get(digest) {
                        Some(disclosure) if disclosure.is_array_element() => {
                            (disclosure.claim_value(), Some(digest))
                        }
                        // No Disclosure behind it: a decoy, which is not an element.
                        _ => continue,
                    },
                };
                let here = position;
                position += 1;
                let wanted = match segment {
                    PathSegment::Index(i) => *i == here,
                    _ => true,
                };
                if !wanted {
                    continue;
                }
                if let Some(digest) = digest {
                    needed.push(digest.to_owned());
                }
                walk(value, rest, needed, held, hits);
                if digest.is_some() {
                    needed.pop();
                }
            }
        }
        _ => {}
    }
}

/// The same navigation over a payload with nothing hidden in it: every claim in
/// its place, no `_sd`, no placeholder. The draft's "as if all were disclosed",
/// without the machinery of [`walk`].
fn navigate<'a>(node: &'a Value, rest: &[PathSegment], found: &mut Vec<&'a Value>) {
    let Some((segment, rest)) = rest.split_first() else {
        found.push(node);
        return;
    };
    match (node, segment) {
        (Value::Object(map), PathSegment::Key(name)) => {
            if let Some(child) = map.get(name) {
                navigate(child, rest, found);
            }
        }
        (Value::Array(items), PathSegment::Index(i)) => {
            if let Some(child) = items.get(*i) {
                navigate(child, rest, found);
            }
        }
        (Value::Array(items), PathSegment::AllElements) => {
            for child in items {
                navigate(child, rest, found);
            }
        }
        _ => {}
    }
}

/// A claim that is not an object or an array. Its value is the same wherever it is
/// read, which a container's is not: a presentation holds fewer of its members.
fn is_scalar(value: &Value) -> bool {
    !matches!(value, Value::Array(_) | Value::Object(_))
}

impl SdJwt {
    /// The digests of the Disclosures that reveal `paths`, and no others, in the
    /// order the token holds them, ready for [`Self::present`] or
    /// [`Self::present_with_key_binding`].
    ///
    /// Every Disclosure between a claim and the signed payload is included, and
    /// nothing else is: not a sibling, not a member of a claim that was named, not an
    /// element that was not. See the module documentation for what a path means,
    /// where an array element is, and the one kind of token this is not exact for.
    ///
    /// # Errors
    ///
    /// [`SdJwtError::ClaimNotInToken`] for a path that selects nothing, naming it.
    ///
    /// [`SdJwtError::KeyBindingAttached`] if this token carries a Key Binding JWT,
    /// as for [`Self::present`].
    ///
    /// Whatever [`Self::disclosed_payload`] refuses the token for, since a token
    /// that cannot be read cannot be chosen from.
    ///
    /// [`SdJwtError::SelectionDisagrees`] if the result fails its own checks, which
    /// is a defect in this function.
    pub fn digests_for_paths(&self, paths: &[ClaimPath]) -> Result<Vec<String>, SdJwtError> {
        if self.has_key_binding() {
            return Err(SdJwtError::KeyBindingAttached);
        }
        let everything = Value::Object(self.disclosed_payload()?);
        let signed = Value::Object(decode_jwt_payload(self.jwt())?);
        let held: HashMap<String, &Disclosure> =
            self.disclosures().iter().map(|d| (d.digest(), d)).collect();

        let mut chosen: HashSet<String> = HashSet::new();
        // What each path reaches in the fully processed payload, kept for the checks
        // made on the presentation below.
        let mut expected: Vec<Vec<&Value>> = Vec::with_capacity(paths.len());
        for path in paths {
            let mut hits = Vec::new();
            walk(&signed, path.segments(), &mut Vec::new(), &held, &mut hits);
            if hits.is_empty() {
                return Err(SdJwtError::ClaimNotInToken(path.to_string()));
            }

            let mut plain = Vec::new();
            navigate(&everything, path.segments(), &mut plain);
            let agrees = hits.len() == plain.len()
                && hits
                    .iter()
                    .zip(&plain)
                    .all(|(hit, plain)| !is_scalar(plain) || hit.value == *plain);
            if !agrees {
                return Err(SdJwtError::SelectionDisagrees(path.to_string()));
            }

            chosen.extend(hits.into_iter().flat_map(|hit| hit.needed));
            expected.push(plain);
        }

        let selected: Vec<Disclosure> = self
            .disclosures()
            .iter()
            .filter(|d| chosen.contains(&d.digest()))
            .cloned()
            .collect();
        let digests = selected.iter().map(Disclosure::digest).collect();

        // Read as a Verifier will. A Disclosure that nothing refers to is refused
        // here, which is what clause 7.2 step 2 asks of a Holder.
        let shown = Value::Object(Self::new(self.jwt().to_owned(), selected).disclosed_payload()?);
        for (path, expected) in paths.iter().zip(&expected) {
            // A position cannot be followed in the presentation, because the
            // elements withheld from it are no longer there to count. So each
            // position is relaxed to every element, and the check is that the
            // scalars the path reached are among what is found.
            let relaxed: Vec<PathSegment> = path
                .segments()
                .iter()
                .map(|segment| match segment {
                    PathSegment::Index(_) => PathSegment::AllElements,
                    other => other.clone(),
                })
                .collect();
            let mut found = Vec::new();
            navigate(&shown, &relaxed, &mut found);
            let present = !found.is_empty()
                && expected
                    .iter()
                    .filter(|value| is_scalar(value))
                    .all(|value| found.contains(value));
            if !present {
                return Err(SdJwtError::SelectionDisagrees(path.to_string()));
            }
        }
        Ok(digests)
    }
}
