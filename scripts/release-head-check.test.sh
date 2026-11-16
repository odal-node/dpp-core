#!/usr/bin/env bash
# Self-test for scripts/release-head-check.sh.
#
# Builds a bare repository standing in for origin and a clone of it, then moves
# the clone and the remote apart in each way a release checkout can drift. Only
# a checkout at origin's main may release: a crates.io publish from anything
# else cannot be taken back.
set -euo pipefail

GATE="$(cd "$(dirname "$0")" && pwd)/release-head-check.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1

origin="$TMP/origin.git"
git init -q --bare -b main "$origin"

# Another contributor's clone, which moves origin's main on.
upstream="$TMP/upstream"
git clone -q "$origin" "$upstream" 2> /dev/null
git -C "$upstream" symbolic-ref HEAD refs/heads/main
commit() { printf '%s\n' "$2" > "$1/change"; git -C "$1" add -A; git -C "$1" commit -qm "$2"; }
commit "$upstream" "first"
git -C "$upstream" push -q origin main

release="$TMP/release"
git clone -q "$origin" "$release"

pass=0
fail=0

# run_case <name> <accept|reject>
run_case() {
    local name="$1" expect="$2"
    local got
    if (cd "$release" && env -i PATH="$PATH" HOME="$TMP" GIT_CONFIG_GLOBAL=/dev/null \
        GIT_CONFIG_NOSYSTEM=1 bash "$GATE" > /dev/null 2>&1); then
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

run_case "main as origin has it" accept

commit "$upstream" "merged after this checkout last pulled"
git -C "$upstream" push -q origin main
run_case "a main that is behind origin" reject

git -C "$release" pull -q --ff-only origin main
run_case "the same main after a pull" accept

commit "$release" "a commit no pull request merged"
run_case "a main that is ahead of origin" reject

git -C "$release" reset -q --hard origin/main
commit "$upstream" "another merge"
git -C "$upstream" push -q origin main
commit "$release" "local work on a stale main"
run_case "a main that has diverged from origin" reject

git -C "$release" reset -q --hard origin/main
git -C "$release" pull -q --ff-only origin main
git -C "$release" switch -q -c feature
commit "$release" "unmerged feature work"
run_case "a feature branch" reject

# A fetch refspec that does not map main leaves origin/main where it was, so
# the check must fetch main into it explicitly rather than trust the config.
git -C "$release" switch -q main
git -C "$release" reset -q --hard origin/main
git -C "$release" config remote.origin.fetch '+refs/heads/other:refs/remotes/origin/other'
commit "$upstream" "merged while the tracking ref was not being updated"
git -C "$upstream" push -q origin main
run_case "a stale origin/main under a refspec that skips main" reject

git -C "$release" remote set-url origin "$TMP/absent.git"
run_case "an origin that cannot be fetched" reject

echo "release-head-check.test: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
