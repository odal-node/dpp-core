#!/usr/bin/env bash
# Self-test for scripts/check-audit-register.sh: proves the register goes red in
# each direction that lets a suppressed advisory rot, against this workspace's
# real dependency graphs and a real, unsuppressed cargo-audit run.
#
# The register is empty while nothing in the graph raises an advisory, so every
# fixture is built from a self-contained base rather than from
# `.cargo/audit.toml`. Each failing case matches its own message: `fail()`
# accumulates, and the base entry is stale by construction (`rsa` is in no graph
# here, so no lockfile raises its advisory), so a bare non-zero exit would prove
# nothing about the check under test.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

SCRATCH="$(mktemp -d)"
trap 'rm -rf "$SCRATCH"' EXIT

PASS=0
FAIL=0

NO_DENY="$SCRATCH/deny-none.toml"
printf '[advisories]\nignore = []\n' > "$NO_DENY"

expect_pass() {
  local name="$1" fixture="$2" deny="${3:-$NO_DENY}"
  if bash scripts/check-audit-register.sh "$fixture" "$deny" > /dev/null 2>&1; then
    echo "ok   - $name"
    PASS=$((PASS + 1))
  else
    echo "FAIL - $name (expected the checker to pass, it did not)"
    FAIL=$((FAIL + 1))
  fi
}

# expect_message <name> <fixture> <deny.toml> <present|absent> <needle>
expect_message() {
  local name="$1" fixture="$2" deny="$3" want="$4" needle="$5" out
  out="$(bash scripts/check-audit-register.sh "$fixture" "$deny" 2>&1 || true)"
  if { [ "$want" = present ] && grep -qF "$needle" <<< "$out"; } \
    || { [ "$want" = absent ] && ! grep -qF "$needle" <<< "$out"; }; then
    echo "ok   - $name"
    PASS=$((PASS + 1))
  else
    echo "FAIL - $name (wanted '$needle' $want)"
    FAIL=$((FAIL + 1))
  fi
}

BASE="$SCRATCH/base.toml"
cat > "$BASE" <<'EOF'
[advisories]
# --- RUSTSEC-2023-0071 -------------------------------------------------------
# crate:      rsa 0.9.10
# path:       none: a test fixture
# class:      not-in-graph
# rationale:  Test fixture. `rsa` is in no graph of this repository.
# anchor:     cargo tree -i rsa --target all
# owner:      maintainer
# recorded:   2026-11-16
# expires:    2999-12-31
# release-on: test fixture, never merged into the real register
ignore = [
    "RUSTSEC-2023-0071",
]
EOF

# variant <name> <sed expression>: the base with one line changed.
variant() {
  local out="$SCRATCH/$1.toml"
  sed "$2" "$BASE" > "$out"
  cmp -s "$BASE" "$out" && { echo "fixture $1 did not change" >&2; exit 1; }
  printf '%s' "$out"
}

# 1. The real register passes as it is.
expect_pass "the real register passes" ".cargo/audit.toml" "deny.toml"

# 2. An entry past its date.
expect_message "a back-dated expires is rejected" \
  "$(variant expired 's/^# expires:    2999-12-31$/# expires:    2020-01-01/')" \
  "$NO_DENY" present "suppression expired on"

# 3. An advisory no lockfile raises is a stale suppression.
expect_message "an advisory no lockfile raises is rejected" "$BASE" "$NO_DENY" present \
  "RUSTSEC-2023-0071: does not appear in a raw"

# 4. A not-in-graph claim for a crate every build resolves.
expect_message "a not-in-graph claim for an in-graph crate is rejected" \
  "$(variant serde 's/^# crate:      rsa 0.9.10$/# crate:      serde_json/')" \
  "$NO_DENY" present "'serde_json' resolves in the . (all features) dependency graph"

# 5. The same claim for a crate only the fuzz workspace resolves: proves every
#    workspace the register covers is read, not only this one.
expect_message "a not-in-graph claim true only outside fuzz/ is rejected" \
  "$(variant fuzzonly 's/^# crate:      rsa 0.9.10$/# crate:      libfuzzer-sys/')" \
  "$NO_DENY" present "'libfuzzer-sys' resolves in the fuzz (all features) dependency graph"

# 6. A dev-only claim for a crate a library reaches through a normal edge.
expect_message "a dev-only claim for a normal dependency is rejected" \
  "$(variant devonly 's/^# crate:      rsa 0.9.10$/# crate:      serde_json/; s/^# class:      not-in-graph$/# class:      dev-only/')" \
  "$NO_DENY" present "'serde_json' resolves in the . (all features) dependency graph"

# 7. A block missing a field.
expect_message "a block without an owner is rejected" \
  "$(variant noowner '/^# owner:/d')" "$NO_DENY" present "missing 'owner:' field"

# 8. An ignore entry that is not an advisory ID.
expect_message "a malformed ignore entry is rejected" \
  "$(variant malformed 's/^    "RUSTSEC-2023-0071",$/    "RUSTSEC-2023-0071", "GHSA-xxxx",/')" \
  "$NO_DENY" present "'GHSA-xxxx' is not a RUSTSEC advisory ID"

# 9. An advisory ignored in deny.toml with no entry in the register.
DENY_OTHER="$SCRATCH/deny-other.toml"
printf '[graph]\nall-features = true\n\n[advisories]\nignore = [\n    { id = "RUSTSEC-2024-0001", reason = "x" },\n]\n\n[bans]\n' > "$DENY_OTHER"
expect_message "an advisory ignored only in deny.toml is rejected" "$BASE" "$DENY_OTHER" present \
  "RUSTSEC-2024-0001: ignored in"

# 10. The same advisory ignored in both is not reported as missing.
DENY_SAME="$SCRATCH/deny-same.toml"
printf '[advisories]\nignore = [\n    { id = "RUSTSEC-2023-0071", reason = "x" },\n]\n' > "$DENY_SAME"
expect_message "an advisory ignored in both files is not reported as missing" "$BASE" "$DENY_SAME" absent \
  "ignored in"

echo ""
echo "check-audit-register.test: $PASS passed, $FAIL failed"
[[ "$FAIL" -eq 0 ]]
