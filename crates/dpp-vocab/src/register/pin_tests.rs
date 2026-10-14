//! Pins: the upstream state each record was read at, in a form a script can check.

use std::collections::HashSet;

use super::test_support::is_iso_date;
use super::*;
use crate::vocabulary::{PinKind, Status};

fn register() -> VocabularyRegister {
    VocabularyRegister::new()
}

fn is_hex(text: &str, len: usize) -> bool {
    text.len() == len
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn is_owner_repo(text: &str) -> bool {
    let mut parts = text.split('/');
    let valid = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    };
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(repo), None) if valid(owner) && valid(repo)
    )
}

/// `checkedOn` is compared as a string against every other date in a record, so
/// the comparison is only meaningful if each of them has the same shape.
#[test]
fn every_checked_on_is_an_iso_date() {
    for v in register().all() {
        assert!(
            is_iso_date(&v.checked_on),
            "'{}' has checkedOn {:?}",
            v.key,
            v.checked_on
        );
    }
}

#[test]
fn every_pin_is_well_formed_for_its_kind() {
    let register = register();
    let mut pinned = 0usize;

    for v in register.all() {
        let mut seen = HashSet::new();
        for pin in &v.pins {
            pinned += 1;
            let at = format!("'{}' pin on {}", v.key, pin.target);

            assert!(
                seen.insert((pin.kind, pin.target.as_str(), pin.path.as_deref())),
                "{at} is listed twice"
            );
            assert!(
                is_iso_date(&pin.observed_on) && pin.observed_on <= v.checked_on,
                "{at} was observed {:?}, which is not a date on or before checkedOn {}",
                pin.observed_on,
                v.checked_on
            );
            match pin.kind {
                PinKind::GithubHead => {
                    assert!(is_owner_repo(&pin.target), "{at} is not owner/repo");
                    assert!(
                        is_hex(&pin.value, 40),
                        "{at} has value {:?}, not a 40-digit commit hash",
                        pin.value
                    );
                    assert!(
                        pin.path
                            .as_deref()
                            .is_none_or(|p| !p.is_empty() && !p.starts_with('/')),
                        "{at} has path {:?}; a path is relative and non-empty",
                        pin.path
                    );
                }
                PinKind::GithubRelease => {
                    assert!(is_owner_repo(&pin.target), "{at} is not owner/repo");
                    assert!(
                        !pin.value.is_empty() && !pin.value.chars().any(char::is_whitespace),
                        "{at} has value {:?}, not a tag",
                        pin.value
                    );
                    assert!(pin.path.is_none(), "{at}: a release has no path");
                }
                PinKind::UrlSha256 => {
                    assert!(
                        pin.target.starts_with("https://")
                            && !pin.target.chars().any(char::is_whitespace),
                        "{at} is not an absolute https URL"
                    );
                    assert!(
                        is_hex(&pin.value, 64),
                        "{at} has value {:?}, not a SHA-256 in lower-case hex",
                        pin.value
                    );
                    assert!(pin.path.is_none(), "{at}: a URL pin has no path");
                }
            }
        }
    }
    assert!(pinned >= 6, "only {pinned} pins were checked");
}

/// A verified record's findings rest on somebody else's publication, which can
/// change without a commit here. One that watches nothing reads the same the day
/// after its source moved as the day before.
#[test]
fn every_verified_record_is_pinned() {
    for v in register()
        .all()
        .iter()
        .filter(|v| v.status == Status::Verified)
    {
        assert!(
            !v.pins.is_empty(),
            "'{}' is verified and pins no upstream state, so nothing can say its source moved",
            v.key
        );
    }
}
