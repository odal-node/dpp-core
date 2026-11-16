#!/usr/bin/env bash
# Refuse to release anything but main as origin has it.
#
# `cargo release` publishes from the local checkout, and a crates.io publish
# cannot be undone, only yanked. `allow-branch = ["main"]` in Cargo.toml refuses
# a release from any other branch by its name, but a local `main` can still be
# behind origin, which publishes code main has since corrected, or ahead of it,
# which publishes commits no pull request merged. This is the first step of the
# pre-release hook (`just pre-release`), so it stops `cargo release` before
# anything is published, and stops a dry run the same way.
#
# Passes only when HEAD is the commit origin's main names after a fetch. In a
# release that bumps the version, the hook runs before cargo-release commits the
# bump, so HEAD is still main's commit.
#
# Input: RELEASE_REMOTE (default origin).
set -euo pipefail

remote="${RELEASE_REMOTE:-origin}"

fail() {
    printf 'release-head-check: %s\n' "$1" >&2
    shift
    for line in "$@"; do printf '%s\n' "$line" >&2; done
    exit 1
}

git fetch --quiet "$remote" main \
    || fail "could not fetch $remote main, so this checkout cannot be compared with it."
main="$(git rev-parse --verify "refs/remotes/$remote/main^{commit}")"
head="$(git rev-parse --verify 'HEAD^{commit}')"

if [ "$head" != "$main" ]; then
    if git merge-base --is-ancestor "$head" "$main"; then
        why="it is behind $remote/main, which has moved on to $main."
    elif git merge-base --is-ancestor "$main" "$head"; then
        why="it has commits $remote/main does not: $(git rev-list --count "$main..$head") ahead of $main."
    else
        why="it has diverged from $remote/main ($main)."
    fi
    fail "HEAD is $head, and $why" \
        "A release publishes main exactly as $remote has it:" \
        "  git switch main && git pull --ff-only $remote main"
fi

echo "release-head-check: HEAD is $remote/main ($main)."
