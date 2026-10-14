//! [`Pin`] — the upstream state a record's findings were read at.

use serde::{Deserialize, Serialize};

/// How a [`Pin`] observes the upstream.
///
/// Each kind is something a script can look up without reading anything, which
/// is the point: a pin cannot say what changed, only **that** something did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum PinKind {
    /// The newest commit on a repository's default branch, or, with a `path`,
    /// the newest commit that touched that path. `value` is the commit hash.
    GithubHead,
    /// The newest published release of a repository. `value` is its tag.
    GithubRelease,
    /// The bytes served at a URL. `value` is their SHA-256, in lower-case hex.
    /// For a document that is published once and replaced, such as a PDF.
    UrlSha256,
}

/// A fact about the upstream at the time a record was read: *"the repository
/// was at this commit"*, *"the release was this tag"*.
///
/// # Why a record needs one
///
/// A `finding` says what a person read and, in prose, when. It cannot be
/// re-checked by anything except another person, so a record whose source has
/// since moved reads exactly like one whose source has not. This crate shipped
/// that state: nine weeks after the last reading, seven of the ten records
/// looked at again had a newer version, a successor or a correction, and nothing
/// had said so.
///
/// A pin gives the check a machine-readable edge. `just vocab-drift` compares
/// each pin with the upstream today and names the records whose source has
/// moved. It cannot say the record is wrong, only that it is **due a reading**.
///
/// # What a pin must not claim
///
/// Only the state the finding rests on. A pin on a path the record never read
/// would report drift in something nobody has looked at, and a pin on a whole
/// repository whose commits are mostly generated files would drift weekly and
/// be ignored. Pin the narrowest thing whose change would matter, such as a
/// release-notes file rather than a model directory full of regenerated
/// artefacts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Pin {
    /// How the upstream is observed.
    pub kind: PinKind,
    /// `owner/repo` for the GitHub kinds; an absolute `https` URL for
    /// [`PinKind::UrlSha256`].
    pub target: String,
    /// Restricts [`PinKind::GithubHead`] to commits that touched this path.
    /// Absent for the other kinds, and for a head taken on the whole repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// What was observed: a commit hash, a tag or a SHA-256, per [`PinKind`].
    pub value: String,
    /// ISO date it was observed. Never later than the record's `checkedOn`.
    pub observed_on: String,
}
