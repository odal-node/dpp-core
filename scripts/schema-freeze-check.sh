#!/usr/bin/env bash
# A released schema version is never changed, except in its prose.
#
# A passport is validated, and its disclosure decided, by the schema version it
# was written under (`VersionedSchemaRegistry`), and every version is published
# under its `$id`. Editing a released version changes what a passport already
# stored, or fetched from another operator, is held to: a tightened rule makes a
# valid one invalid on its next read, a loosened one accepts what the version
# never allowed, and a changed `x-disclosure` serves a field to an audience it was
# not written for. None of that fails a build. A new rule is a new version.
#
# The frozen fixtures in `crates/dpp-domain/tests/schema_compat.rs` catch the
# first case only, for the documents they hold. This checks the file.
#
# For every schema file the latest release tag holds, the file with the same
# `$id` must still exist and must match it once the prose is set aside. Prose is
# `title`, `description`, `$comment` and `examples` where they annotate a schema:
# a property named `description` is not prose, and neither is anything inside
# `enum`, `const`, `default` or an `x-*` value. Key order does not count. So a
# reworded description passes, and so does a file moved with its `$id`; a changed
# keyword, a changed `x-*` annotation and a deleted file do not.
#
# Inputs: SCHEMA_DIRS (default: dpp-domain's `schemas` and `document-schemas`),
# RELEASE_REF (default: the newest `v*` tag reachable from HEAD). Compares that
# ref with the working tree. Needs the tags (a CI checkout with fetch-depth: 0).
set -euo pipefail

dirs="${SCHEMA_DIRS:-crates/dpp-domain/schemas crates/dpp-domain/document-schemas}"

fail() {
    printf '::error::%s\n' "$1" >&2
    shift
    for line in "$@"; do printf '%s\n' "$line" >&2; done
    exit 1
}

release="${RELEASE_REF:-$(git describe --tags --abbrev=0 --match 'v[0-9]*' HEAD 2> /dev/null || true)}"
[ -n "$release" ] || fail "no release tag is reachable from HEAD, so there is nothing to compare with." \
    "Fetch the tags (actions/checkout with fetch-depth: 0)."
git rev-parse --verify --quiet "$release^{commit}" > /dev/null \
    || fail "$release is not in this checkout."

# The schema with its prose set aside, keys sorted, on one line.
# shellcheck disable=SC2016 # `$comment` is a JSON key for jq, not a shell variable.
normalize='
def strip:
  if type == "object" then
    with_entries(
      if (.key | IN("title", "description", "$comment", "examples")) then empty
      elif (.key | IN("enum", "const", "default")) or (.key | startswith("x-")) then .
      elif (.key | IN("properties", "patternProperties", "$defs", "definitions", "dependentSchemas"))
        and (.value | type) == "object"
      then .value |= map_values(strip)
      else .value |= strip
      end)
  elif type == "array" then map(strip)
  else . end;
strip'

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# `$id` -> path, for every schema in the working tree.
declare -A current
for dir in $dirs; do
    [ -d "$dir" ] || continue
    while IFS= read -r file; do
        id="$(jq -r '."$id" // empty' "$file")" || fail "$file is not valid JSON."
        current["${id:-$file}"]="$file"
    done < <(find "$dir" -name '*.json' -type f | sort)
done

released=0
bad=0
# shellcheck disable=SC2086 # the dirs are a list of paths.
while IFS= read -r path; do
    released=$((released + 1))
    old="$work/old.json"
    git show "$release:$path" > "$old"
    id="$(jq -r '."$id" // empty' "$old")"
    now="${current[${id:-$path}]:-}"
    if [ -z "$now" ]; then
        printf '::error::%s (%s) was released in %s and is gone. A released schema version stays, so a stored passport that names it can still be read.\n' \
            "$path" "${id:-no \$id}" "$release" >&2
        bad=$((bad + 1))
        continue
    fi
    jq -S "$normalize" "$old" > "$work/a.json"
    jq -S "$normalize" "$now" > "$work/b.json"
    if ! cmp -s "$work/a.json" "$work/b.json"; then
        printf '::error::%s (%s) was released in %s, and more than its prose has changed since:\n' \
            "$now" "${id:-no \$id}" "$release" >&2
        diff -u --label "$release" --label "working tree" "$work/a.json" "$work/b.json" | head -40 >&2 || true
        bad=$((bad + 1))
    fi
done < <(git ls-tree -r --name-only "$release" -- $dirs | grep '\.json$')

[ "$released" -gt 0 ] || fail "$release holds no schema under: $dirs. A path change here would make this check pass while checking nothing."
if [ "$bad" -ne 0 ]; then
    fail "$bad released schema version(s) changed. Put the change in a new version, and leave the released one as it was."
fi
echo "schema-freeze-check: $released schema files released in $release are unchanged but for their prose."
