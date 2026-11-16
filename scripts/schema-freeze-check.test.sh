#!/usr/bin/env bash
# Self-test for scripts/schema-freeze-check.sh.
#
# Releases one schema in a scratch repository, then changes it in each way the
# gate must tell apart: prose, which passes, and what a stored passport is held
# to, which does not. Each case starts from the released tree.
#
# shellcheck disable=SC2016 # jq filters are single-quoted on purpose: `$id` is a JSON key.
set -euo pipefail

GATE="$(cd "$(dirname "$0")" && pwd)/schema-freeze-check.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1

repo="$TMP/repo"
git init -q -b main "$repo"
mkdir -p "$repo/schemas/widget" "$repo/document-schemas"

cat > "$repo/schemas/widget/v1.0.0.json" <<'JSON'
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://example.test/widget-v1.0.0.json",
  "title": "Widget (v1.0.0)",
  "description": "A widget.",
  "type": "object",
  "required": ["mass"],
  "properties": {
    "mass": { "type": "number", "minimum": 0, "description": "In grams.", "x-disclosure": "public" },
    "description": { "type": "string", "x-disclosure": "public" },
    "colour": { "enum": ["red", "blue"], "examples": ["red"] }
  },
  "dependencies": {
    "description": ["mass"],
    "colour": { "required": ["mass"], "description": "A coloured widget states its mass." }
  }
}
JSON
git -C "$repo" add -A
git -C "$repo" commit -qm "release 0.1.0"
git -C "$repo" tag v0.1.0

released=schemas/widget/v1.0.0.json
moved=document-schemas/widget/v1.0.0.json

# The changes, each run inside the repository.
untouched() { :; }
edit() { jq "$1" "$released" > t.json && mv t.json "$released"; }
move_then() { mkdir -p document-schemas/widget && mv "$released" "$moved" && jq "$1" "$moved" > t.json && mv t.json "$moved"; }
new_version() {
    jq '."$id" = "https://example.test/widget-v1.1.0.json" | .required += ["colour"]' \
        "$released" > schemas/widget/v1.1.0.json
}
delete() { rm "$released"; }
corrupt() { printf '{' > "$released"; }

pass=0
fail=0

# run_case <name> <accept|reject> <change> [args]: apply the change to a fresh
# copy of the released tree, then ask the gate.
run_case() {
    local name="$1" expect="$2"
    shift 2
    git -C "$repo" checkout -q -- .
    git -C "$repo" clean -qfd
    (cd "$repo" && "$@")
    local got
    if (cd "$repo" && env -i PATH="$PATH" SCHEMA_DIRS="schemas document-schemas" \
        GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 bash "$GATE" > /dev/null 2>&1); then
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

run_case "the released schema, untouched" accept untouched
run_case "a new version beside it" accept new_version
run_case "a reworded description and title" accept edit '.description = "Reworded." | .title = "Widget, v1.0.0"'
run_case "a reworded description on a property" accept edit '.properties.mass.description = "Mass in grams."'
run_case "a \$comment and examples added" accept edit '."$comment" = "note" | .properties.mass.examples = [1]'
run_case "keys reordered" accept edit '. as $s | ($s | keys | reverse) as $k | reduce $k[] as $key ({}; .[$key] = $s[$key])'
run_case "moved with its \$id, prose reworded" accept move_then '.description = "Moved."'
run_case "a field made required" reject edit '.required += ["colour"]'
run_case "a minimum changed" reject edit '.properties.mass.minimum = 1'
run_case "a type changed" reject edit '.properties.mass.type = "string"'
run_case "a disclosure annotation changed" reject edit '.properties.mass."x-disclosure" = "restricted"'
run_case "a property named description removed" reject edit 'del(.properties.description)'
run_case "an enum member added" reject edit '.properties.colour.enum += ["green"]'
run_case "a dependency on a property named description changed" reject edit '.dependencies.description = ["colour"]'
run_case "a dependency renamed" reject edit '.dependencies.title = .dependencies.description | del(.dependencies.description)'
run_case "prose inside a schema dependency reworded" accept edit '.dependencies.colour.description = "Reworded."'
run_case "moved with its \$id, a rule changed" reject move_then '.required = []'
run_case "the released file deleted" reject delete
run_case "the \$id changed" reject edit '."$id" = "https://example.test/other.json"'
run_case "invalid JSON" reject corrupt

# No release tag at all: nothing to compare with must not read as nothing changed.
git -C "$repo" tag -d v0.1.0 > /dev/null
run_case "no release tag" reject untouched

echo "schema-freeze-check.test: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
