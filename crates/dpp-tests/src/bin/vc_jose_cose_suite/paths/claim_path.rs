//! Claim paths read from the suite's selective-disclosure input.

use dpp_crypto::sd_jwt::{ClaimPath, PathSegment};
use serde_json::Value;

/// The `--sd` value, a JSON array of the suite's paths, as `ClaimPath`s.
pub fn parse_sd(text: &str) -> Result<Vec<ClaimPath>, String> {
    let value: Value = serde_json::from_str(text).map_err(|e| format!("--sd is not JSON: {e}"))?;
    let items = value
        .as_array()
        .ok_or("--sd is not a JSON array of strings")?;
    items
        .iter()
        .map(|item| {
            let path = item
                .as_str()
                .ok_or("--sd holds a value that is not a string")?;
            claim_path(path)
        })
        .collect()
}

/// One of the suite's paths: members separated by `.`, each with any number of `[n]`.
pub fn claim_path(path: &str) -> Result<ClaimPath, String> {
    let bad = |why: &str| format!("the path `{path}` {why}");
    let mut segments = Vec::new();
    for part in path.split('.') {
        let name_end = part.find('[').unwrap_or(part.len());
        let (name, mut indices) = part.split_at(name_end);
        if name.is_empty() || name.contains(']') {
            return Err(bad("has an empty or malformed member name"));
        }
        segments.push(PathSegment::Key(name.to_owned()));
        while !indices.is_empty() {
            let (inside, rest) = indices
                .strip_prefix('[')
                .and_then(|after| after.split_once(']'))
                .ok_or_else(|| bad("has an unclosed or stray bracket"))?;
            let position = inside
                .parse::<usize>()
                .ok()
                .filter(|_| inside.bytes().all(|b| b.is_ascii_digit()))
                .ok_or_else(|| bad("has a position that is not a non-negative integer"))?;
            segments.push(PathSegment::Index(position));
            indices = rest;
        }
    }
    ClaimPath::new(segments).map_err(|e| bad(&e.to_string()))
}
