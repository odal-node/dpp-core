#!/usr/bin/env bash
# Self-test for scripts/vocabulary-check.sh.
#
# Builds a tree that uses the word in every way that is allowed, then adds each
# use the rule refuses. The gate reads crates/*/src and the schema directories
# relative to VOCABULARY_ROOT.
#
# shellcheck disable=SC2016 # each case is a string run inside its own tree, where `$lib` expands.
set -euo pipefail

GATE="$(cd "$(dirname "$0")" && pwd)/vocabulary-check.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

pass=0
fail=0

# Every allowed use: the refusal of the old status, a comment that names the
# word, the keystore's archived keys, a clause 4.2 type, and a test that sends
# the old status to watch it refused.
scaffold() {
    local root="$1"
    mkdir -p "$root/crates/dom/src/status" "$root/crates/dom/schemas/widget" "$root/crates/key/src"
    cat > "$root/crates/dom/src/status/lifecycle.rs" <<'EOF'
// "archived" was the old name of `retired`.
fn parse(s: &str) -> Result<Status, Error> {
    match s {
        "retired" => Ok(Status::Retired),
        "archived" => Err(Error::custom("this status is now `retired`")),
        other => Err(Error::unknown(other)),
    }
}
pub trait ArchivedVersionPort {}
EOF
    mkdir -p "$root/crates/dom/src/status/tests"
    cat > "$root/crates/dom/src/status/tests/status.rs" <<'EOF'
fn old_status_is_refused() { parse("archived").unwrap_err(); }
EOF
    cat > "$root/crates/key/src/rotation.rs" <<'EOF'
fn archived_id(key_id: &str) -> String { format!("{key_id}#archived-1") }
EOF
    printf '%s\n' '{ "enum": ["draft", "retired"] }' > "$root/crates/dom/schemas/widget/v1.0.0.json"
}

run_case() {
    local name="$1" expect="$2" mutate="$3"
    local root="$TMP/$name"
    scaffold "$root"
    (cd "$root" && eval "$mutate")
    local got
    if VOCABULARY_ROOT="$root" bash "$GATE" > /dev/null 2>&1; then
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

lib=crates/dom/src/status/lifecycle.rs
export lib

run_case "every_allowed_use_is_accepted" accept "true"
run_case "a_status_sent_as_archived_is_refused" reject \
    'printf "%s\n" "fn wire() -> &'"'"'static str { \"archived\" }" >> "$lib"'
run_case "a_path_segment_archive_is_refused" reject \
    'printf "%s\n" "const SEGMENT: &str = \"archive\";" >> "$lib"'
run_case "a_serde_rename_is_refused" reject \
    'printf "%s\n" "#[serde(rename = \"archivedAt\")]" >> "$lib"'
run_case "a_serde_alias_is_refused" reject \
    'printf "%s\n" "#[serde(alias = \"archived\")]" >> "$lib"'
run_case "a_variant_named_archived_is_refused" reject \
    'printf "%s\n" "pub enum Status { Retired, Archived }" >> "$lib"'
run_case "a_schema_value_archived_is_refused" reject \
    'printf "%s\n" "{ \"enum\": [\"draft\", \"archived\"] }" > crates/dom/schemas/widget/v1.0.0.json'
run_case "the_refusal_removed_is_refused" reject \
    'grep -v "=> Err(Error::custom" "$lib" > t && mv t "$lib"'
run_case "the_word_in_a_test_file_is_accepted" accept \
    'printf "%s\n" "fn t() { let _ = \"archive\"; }" >> crates/dom/src/status/tests/status.rs'
run_case "the_word_in_an_inline_test_module_is_accepted" accept \
    'printf "%s\n" "#[cfg(test)]" "mod tests { fn t() { let _ = parse(\"archived\"); } }" >> "$lib"'
run_case "the_word_in_a_message_is_accepted" accept \
    'printf "%s\n" "const KEPT: &str = \"Archived keys must be retained\";" >> "$lib"'
run_case "a_rename_split_across_lines_is_refused" reject \
    'printf "%s\n" "#[serde(" "    rename =" "        \"archivedAt\"" ")]" "struct S;" >> "$lib"'
run_case "an_alias_split_across_lines_is_refused" reject \
    'printf "%s\n" "#[serde(" "    alias" "        = \"archivedOn\"" ")]" "struct T;" >> "$lib"'
run_case "a_value_above_an_inline_test_module_is_still_refused" reject \
    'printf "%s\n" "const SENT: &str = \"archived\";" "#[cfg(test)]" "mod tests {}" >> "$lib"'

echo "vocabulary-check.test: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
