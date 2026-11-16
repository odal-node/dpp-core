#!/usr/bin/env bash
# Refuse a release tag that does not name the commit crates.io published.
#
# `cargo package` writes `.cargo_vcs_info.json` into every `.crate`, recording
# the commit it was packaged from and whether the working tree was dirty, and
# crates.io serves the file exactly as published. A release published from one
# commit and tagged on another would otherwise pass every check that reads only
# git: the tag can be on main and state the right version while the crates hold
# a different tree.
#
# A tag passes when, for every crate this workspace publishes, the `.crate` at
# the tag's version exists, records the tagged commit as `git.sha1`, and is not
# marked dirty.
#
# Inputs: RELEASE_TAG (e.g. v0.24.0) and RELEASE_COMMIT (the commit it points
# at). CRATES_URL is where the `.crate` files are read from (default
# https://static.crates.io/crates). RELEASE_CRATES names the crates, separated
# by spaces; by default every workspace member that publishes, read with
# `cargo metadata`. CRATES_RETRIES (default 5) is how often a missing `.crate`
# is asked for again. The self-test sets all three.
set -euo pipefail

: "${RELEASE_TAG:?RELEASE_TAG is the tag being released, e.g. v0.24.0}"
: "${RELEASE_COMMIT:?RELEASE_COMMIT is the commit the tag points at}"
crates_url="${CRATES_URL:-https://static.crates.io/crates}"
retries="${CRATES_RETRIES:-5}"

fail() {
    printf '::error::%s\n' "$1" >&2
    shift
    for line in "$@"; do printf '%s\n' "$line" >&2; done
    exit 1
}

case "$RELEASE_TAG" in
    v?*) ;;
    *) fail "the tag '$RELEASE_TAG' does not start with 'v'." ;;
esac
version="${RELEASE_TAG#v}"

if [ -n "${RELEASE_CRATES:-}" ]; then
    crates="$RELEASE_CRATES"
else
    # `publish = false` reads as an empty list; a publishable crate has none.
    crates="$(cargo metadata --no-deps --format-version 1 --locked \
        | jq -r '.packages[] | select(.publish != []) | .name')"
fi
[ -n "${crates//[[:space:]]/}" ] || fail "no published crate to check."

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

bad=0
checked=0
for name in $crates; do
    file="$work/$name-$version.crate"
    # A tag pushed by `cargo release` follows the upload by seconds, so give
    # crates.io a minute before calling a crate missing.
    if ! curl -fsSL --retry "$retries" --retry-delay 12 --retry-all-errors \
        -A "dpp-core release-crates-check" \
        -o "$file" "$crates_url/$name/$name-$version.crate"; then
        printf '::error::%s %s is not on crates.io.\n' "$name" "$version" >&2
        bad=$((bad + 1))
        continue
    fi
    if ! vcs="$(tar -xzOf "$file" "$name-$version/.cargo_vcs_info.json" 2> /dev/null)"; then
        printf '::error::%s %s records no commit (.cargo_vcs_info.json is missing).\n' \
            "$name" "$version" >&2
        bad=$((bad + 1))
        continue
    fi
    sha="$(jq -r '.git.sha1 // empty' <<< "$vcs")"
    dirty="$(jq -r '.git.dirty // false' <<< "$vcs")"
    if [ "$sha" != "$RELEASE_COMMIT" ]; then
        printf '::error::%s %s was published from %s, not the tagged commit %s.\n' \
            "$name" "$version" "${sha:-an unrecorded commit}" "$RELEASE_COMMIT" >&2
        bad=$((bad + 1))
    elif [ "$dirty" != "false" ]; then
        printf '::error::%s %s was published from a dirty working tree at %s.\n' \
            "$name" "$version" "$sha" >&2
        bad=$((bad + 1))
    fi
    checked=$((checked + 1))
done

if [ "$bad" -ne 0 ]; then
    fail "$bad crate(s) do not match $RELEASE_TAG." \
        "If the tag is on the wrong commit, delete it here and on origin and tag the commit" \
        "the crates record:" \
        "  git tag -d $RELEASE_TAG && git push origin :refs/tags/$RELEASE_TAG"
fi
echo "release-crates-check: $checked crates at $version were published from $RELEASE_COMMIT."
