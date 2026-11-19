#!/usr/bin/env bash
# A release tag names what was published, so refuse one that is not a release of
# main.
#
# `cargo release` publishes the crates from a local checkout, and the `v*` tag
# that names the release can be made by hand afterwards. `git tag` without a
# commit tags whatever is checked out, so a checkout left on a feature branch
# tags that branch. The release page, the compare links in the release notes and
# anyone who checks out the tag would then read code that is not what crates.io
# holds. Pushing a `v*` tag runs `.github/workflows/release-tag.yml`, which runs
# this check and then `scripts/release-crates-check.sh`.
#
# A tag passes when both hold:
#
#   - its commit is on main: main itself, or an ancestor of it. A release is cut
#     from main (docs/governance/RELEASE.md), so this holds for every release the
#     process describes;
#   - it is `v` followed by the commit's own `[workspace.package] version`. The
#     version has one home, Cargo.toml, and a tag that disagrees with it names a
#     release the code does not say it is.
#
# Inputs: RELEASE_TAG (the tag's name, e.g. v0.24.0), RELEASE_COMMIT (the commit
# it points at) and MAIN_REF (default origin/main). Run from the root of a
# checkout with full history. A shallow one has no main to compare with, and the
# check then fails rather than passing unchecked.
set -euo pipefail

: "${RELEASE_TAG:?RELEASE_TAG is the tag being released, e.g. v0.24.0}"
: "${RELEASE_COMMIT:?RELEASE_COMMIT is the commit the tag points at}"
main_ref="${MAIN_REF:-origin/main}"

fail() {
    printf '::error::%s\n' "$1" >&2
    shift
    for line in "$@"; do printf '%s\n' "$line" >&2; done
    exit 1
}

case "$RELEASE_TAG" in
    v?*) ;;
    *) fail "the tag '$RELEASE_TAG' does not start with 'v'." \
        "A release is tagged v<version>, with the version Cargo.toml states." ;;
esac
tag_version="${RELEASE_TAG#v}"

git rev-parse --verify --quiet "$main_ref^{commit}" > /dev/null \
    || fail "$main_ref is not in this checkout, so the tag cannot be checked against it." \
        "The workflow must fetch main (actions/checkout with fetch-depth: 0)."
git rev-parse --verify --quiet "$RELEASE_COMMIT^{commit}" > /dev/null \
    || fail "the tagged commit $RELEASE_COMMIT is not in this checkout."

if ! git merge-base --is-ancestor "$RELEASE_COMMIT" "$main_ref"; then
    fail "$RELEASE_TAG points at $RELEASE_COMMIT, which is not on main." \
        "A release is tagged on the commit it was published from, named by hash:" \
        "  git tag -a $RELEASE_TAG <release-commit-sha> -m \"Release version $tag_version\"" \
        "Delete this tag here and on origin, then tag that commit:" \
        "  git tag -d $RELEASE_TAG && git push origin :refs/tags/$RELEASE_TAG"
fi

cargo_toml="$(git show "$RELEASE_COMMIT:Cargo.toml" 2> /dev/null)" \
    || fail "the tagged commit $RELEASE_COMMIT has no Cargo.toml."
# The version under [workspace.package] only: a `version` key in another table
# (a dependency, a [package]) is not the release's. TOML lets a line be indented
# and a header carry spaces inside its brackets and a comment after them, and
# refusing a release over any of those would be a false alarm.
cargo_version="$(awk '
    /^[[:space:]]*\[/ {
        in_pkg = ($0 ~ /^[[:space:]]*\[[[:space:]]*workspace\.package[[:space:]]*\][[:space:]]*(#.*)?$/)
        next
    }
    in_pkg && !found && /^[[:space:]]*version[[:space:]]*=/ {
        sub(/^[[:space:]]*version[[:space:]]*=[[:space:]]*"/, "")
        sub(/".*$/, "")
        print
        found = 1
    }' <<< "$cargo_toml")"
[ -n "$cargo_version" ] \
    || fail "the tagged commit's Cargo.toml states no [workspace.package] version."

if [ "$tag_version" != "$cargo_version" ]; then
    fail "$RELEASE_TAG does not match the version its commit's Cargo.toml states, $cargo_version." \
        "Tag the commit the release was published from: its Cargo.toml states the version." \
        "Delete this tag here and on origin first:" \
        "  git tag -d $RELEASE_TAG && git push origin :refs/tags/$RELEASE_TAG"
fi

echo "release-tag-check: $RELEASE_TAG is $RELEASE_COMMIT, on $main_ref, and Cargo.toml states $cargo_version."
