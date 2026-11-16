#!/usr/bin/env bash
# Self-test for scripts/release-crates-check.sh.
#
# Builds `.crate` files the way crates.io serves them, with and without a
# matching `.cargo_vcs_info.json`, and asks the gate about each set through a
# file:// URL. The refusals are the cases that matter: the gate runs only when a
# tag is pushed, after the crates are published, so one that accepts everything
# would read green at the one moment it is needed.
set -euo pipefail

GATE="$(cd "$(dirname "$0")" && pwd)/release-crates-check.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

released=1111111111111111111111111111111111111111
other=2222222222222222222222222222222222222222

# publish <registry> <crate> <version> <.cargo_vcs_info.json contents, or "none">
publish() {
    local registry="$1" name="$2" version="$3" vcs="$4"
    local src="$TMP/src/$registry/$name-$version"
    mkdir -p "$src" "$TMP/$registry/$name"
    printf '[package]\nname = "%s"\nversion = "%s"\n' "$name" "$version" > "$src/Cargo.toml"
    [ "$vcs" = none ] || printf '%s' "$vcs" > "$src/.cargo_vcs_info.json"
    tar -czf "$TMP/$registry/$name/$name-$version.crate" -C "$TMP/src/$registry" "$name-$version"
}

vcs() { printf '{"git":{"sha1":"%s"},"path_in_vcs":"crates/x"}' "$1"; }
dirty_vcs() { printf '{"git":{"sha1":"%s","dirty":true},"path_in_vcs":"crates/x"}' "$1"; }

publish good a 0.2.0 "$(vcs $released)"
publish good b 0.2.0 "$(vcs $released)"

publish one_other a 0.2.0 "$(vcs $released)"
publish one_other b 0.2.0 "$(vcs $other)"

publish one_missing a 0.2.0 "$(vcs $released)"

publish one_dirty a 0.2.0 "$(vcs $released)"
publish one_dirty b 0.2.0 "$(dirty_vcs $released)"

publish no_vcs a 0.2.0 "$(vcs $released)"
publish no_vcs b 0.2.0 none

pass=0
fail=0

# run_case <name> <accept|reject> <registry> <tag> <commit> [crates]
run_case() {
    local name="$1" expect="$2" registry="$3" tag="$4" commit_id="$5" crates="${6:-a b}"
    local got
    if env -i PATH="$PATH" RELEASE_TAG="$tag" RELEASE_COMMIT="$commit_id" \
        CRATES_URL="file://$TMP/$registry" RELEASE_CRATES="$crates" CRATES_RETRIES=0 \
        bash "$GATE" > /dev/null 2>&1; then
        got=accept
    else
        got=reject
    fi
    if [ "$got" = "$expect" ]; then
        echo "ok:   $name ($expect)"
        pass=$((pass + 1))
    else
        echo "FAIL: $name — gate said $got, expected $expect"
        fail=$((fail + 1))
    fi
}

run_case "every crate records the tagged commit" accept good v0.2.0 "$released"
run_case "a tag on a commit the crates do not record" reject good v0.2.0 "$other"
run_case "one crate published from another commit" reject one_other v0.2.0 "$released"
run_case "one crate not published at the version" reject one_missing v0.2.0 "$released"
run_case "one crate published from a dirty tree" reject one_dirty v0.2.0 "$released"
run_case "one crate with no recorded commit" reject no_vcs v0.2.0 "$released"
run_case "a version nothing was published at" reject good v0.3.0 "$released"
run_case "a tag without its v" reject good 0.2.0 "$released"
run_case "no crate to check" reject good v0.2.0 "$released" " "

echo "release-crates-check.test: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
