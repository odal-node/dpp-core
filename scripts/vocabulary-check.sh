#!/usr/bin/env bash
# Keep "archive" meaning one thing on the wire.
#
# EN 18221 clause 4.2 uses "archiving" for the retention of historical versions
# of a passport that is still live, which `ArchivedVersionPort` serves. This
# workspace once spent the word on a terminal lifecycle status too, now
# `PassportStatus::Retired`, and the ESPR Art. 10(4) back-up copy is
# `BackupCopyPort`. While two things wore the word, anyone mapping a passport
# onto the standard by name read a status as clause 4.2 archiving. The rename
# holds only while nothing brings the word back as a value: nothing fails if the
# next status, or a schema enum, is called `archived`. This is what fails.
#
# The rule, over production source (`crates/*/src`, tests aside) and the
# published schemas:
#
#   1. no `"archive"` or `"archived"` string literal, except as the pattern of a
#      match arm that answers it with `Err(`: refusing the old status on input is
#      the one use the word has here, and `PassportStatus`'s deserializer does it;
#   2. no serde `rename` or `alias` containing "archiv";
#   3. no identifier that is the bare word `Archive` or `Archived`, which is what
#      a variant or a type named for the old status would be;
#   4. no `"archive"` or `"archived"` value in a schema file.
#
# Not checked, because each is qualified by its own object: `ArchivedVersionPort`
# and its archived versions (clause 4.2 itself), the keystore's archived keys,
# and "archiving" in a trusted list's service names (ETSI's own term).
set -euo pipefail

ROOT="${VOCABULARY_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
cd "$ROOT"

status=0

# Production Rust: crates/*/src without test modules and files. A comment line is
# prose, which may name the word to explain it.
production() {
    grep -rn --include='*.rs' -E "$1" crates/*/src 2> /dev/null \
        | grep -v -E '^[^:]+/(tests|oracle_tests)/|^[^:]+/tests\.rs:|^[^:]+_tests\.rs:' \
        | grep -v -E '^[^:]+:[0-9]+:[[:space:]]*//' || true
}

hits="$(production '"archived?"' | grep -v -E '"archived?"[[:space:]]*=>[[:space:]]*Err\(' || true)"
if [ -n "$hits" ]; then
    printf '%s\n' "$hits"
    echo "ERROR: \"archive\" or \"archived\" is used as a value. The lifecycle status is \`retired\`; \"archive\" means EN 18221 clause 4.2 version archiving. A literal may appear only where it is refused (\`\"archived\" => Err(\`)."
    status=1
fi

hits="$(production '(rename|alias)[[:space:]]*=[[:space:]]*"[^"]*[Aa]rchiv')"
if [ -n "$hits" ]; then
    printf '%s\n' "$hits"
    echo "ERROR: a serde name containing \"archiv\" puts the word on the wire. See the rule at the top of scripts/vocabulary-check.sh."
    status=1
fi

hits="$(production '(^|[^A-Za-z0-9_])Archived?([^A-Za-z0-9_]|$)')"
if [ -n "$hits" ]; then
    printf '%s\n' "$hits"
    echo "ERROR: an identifier named bare \`Archive\` or \`Archived\`. The status is \`Retired\`; a clause 4.2 type names what is archived (\`ArchivedVersionPort\`)."
    status=1
fi

hits="$(grep -rn -E '"archived?"' crates/*/schemas crates/*/document-schemas 2> /dev/null || true)"
if [ -n "$hits" ]; then
    printf '%s\n' "$hits"
    echo "ERROR: a schema carries \"archive\" or \"archived\" as a value. See the rule at the top of scripts/vocabulary-check.sh."
    status=1
fi

# The refusal must still be there: without it, a stored `archived` would read as
# an unknown variant and nothing here would notice the rule had lost its anchor.
if ! production '"archived"[[:space:]]*=>[[:space:]]*Err\(' | grep -q .; then
    echo "ERROR: no match arm refuses \"archived\" any more. PassportStatus's deserializer answers the old status with the reason it was renamed; restore it, or rewrite this check."
    status=1
fi

exit "$status"
