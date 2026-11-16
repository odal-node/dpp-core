#!/usr/bin/env bash
# Enforces the structured-header contract on every `.cargo/audit.toml` ignore
# entry. A suppressed advisory cannot outlive the argument that suppressed
# it: each entry bundles a reachability claim, an availability claim, and an
# implicit expiry — left as prose, none of the three ever gets re-checked.
# This makes all three mechanical.
#
# Each ignore entry must be preceded by a block of this shape:
#
#   # --- RUSTSEC-XXXX-XXXX ------------------------------------------------
#   # crate:      <name> <version>
#   # path:       <dependency path, free text>
#   # class:      not-in-graph | dev-only | build-time-only |
#   #             reachable-but-mitigated | reachable-accepted
#   # rationale:  <free text, may wrap on continuation lines>
#   # anchor:     <path::SYMBOL that makes `class` true, or "—" for
#   #             reachable-accepted, which has no automated check>
#   # owner:      <who signed off>
#   # recorded:   <YYYY-MM-DD>
#   # expires:    <YYYY-MM-DD>
#   # release-on: <the condition that would let this entry be deleted>
#
# Checks (any failure is fatal):
#   1. every ID in [advisories].ignore has a block, every block has all nine
#      fields, and every block's ID is itself in the ignore list
#   2. no block's `expires` date has passed
#   3. no ignored ID is stale — it must still be reported by a "raw" cargo
#      audit run (no .cargo/audit.toml in scope, i.e. outside this repo)
#   4. each block's `class` claim actually holds against the dependency graph
#      (not-in-graph / dev-only / build-time-only / reachable-but-mitigated);
#      reachable-accepted has no automated check by design (manual sign-off)
#   5. no advisory is ignored in deny.toml without an entry here: `just audit`
#      runs cargo-audit and cargo-deny alike, and an ID ignored only in
#      deny.toml would be a suppression with none of the above
#
# The one register covers all three workspaces (`.`, `plugins/`, `fuzz/`):
# `just audit` reads each lockfile with this `.cargo/audit.toml`. So an entry is
# stale only when no lockfile raises it, and a graph claim must hold in every
# workspace with every feature on, since a consumer of a published crate can
# turn on any of them.
#
# Usage: scripts/check-audit-register.sh [path/to/audit.toml] [path/to/deny.toml]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

AUDIT_TOML="${1:-.cargo/audit.toml}"
DENY_TOML="${2:-deny.toml}"
WORKSPACES=(. plugins fuzz)
STATUS=0
fail() { echo "FAIL: $*" >&2; STATUS=1; }

REQUIRED_FIELDS=(crate path class rationale anchor owner recorded expires release-on)

# ── Parse the file into BLOCK / FIELD / IGNORED records ─────────────────────
PARSED="$(awk '
  /^# --- RUSTSEC-[0-9]{4}-[0-9]{4} / {
    match($0, /RUSTSEC-[0-9]{4}-[0-9]{4}/)
    id = substr($0, RSTART, RLENGTH)
    print "BLOCK\t" id
    next
  }
  id != "" && /^# [a-z][a-z-]*:[ \t]/ {
    line = $0
    sub(/^# /, "", line)
    idx = index(line, ":")
    key = substr(line, 1, idx - 1)
    val = substr(line, idx + 1)
    sub(/^[ \t]+/, "", val)
    sub(/[ \t]+$/, "", val)
    print "FIELD\t" id "\t" key "\t" val
    next
  }
  # The array may be written on one line or many. Every quoted entry on it is
  # read, so an ID cannot hide in a layout this parser skips. An entry that is not
  # a RUSTSEC ID is reported rather than dropped. TOML allows both basic ("...")
  # and literal (\047...\047) strings, so both are read; \047 is a single quote.
  /^[ \t]*ignore[ \t]*=[ \t]*\[/ { inlist = 1 }
  inlist {
    rest = $0
    sub(/#.*/, "", rest)
    while (match(rest, /"[^"]*"|\047[^\047]*\047/)) {
      entry = substr(rest, RSTART + 1, RLENGTH - 2)
      if (entry ~ /^RUSTSEC-[0-9]{4}-[0-9]{4}$/) print "IGNORED\t" entry
      else print "MALFORMED\t" entry
      rest = substr(rest, RSTART + RLENGTH)
    }
    if ($0 ~ /\]/) inlist = 0
    next
  }
' "$AUDIT_TOML")"

declare -A FIELD
BLOCK_IDS=()
IGNORED_IDS=()

while IFS=$'\t' read -r kind a b c; do
  case "$kind" in
    BLOCK) BLOCK_IDS+=("$a") ;;
    FIELD) FIELD["$a|$b"]="$c" ;;
    IGNORED) IGNORED_IDS+=("$a") ;;
    MALFORMED) fail "[advisories].ignore entry '$a' is not a RUSTSEC advisory ID" ;;
  esac
done <<< "$PARSED"

contains() { local needle="$1"; shift; for x in "$@"; do [[ "$x" == "$needle" ]] && return 0; done; return 1; }

# ── Check 1: every ignored ID has a complete block, and vice versa ─────────
for id in "${IGNORED_IDS[@]}"; do
  contains "$id" "${BLOCK_IDS[@]}" || fail "$id: in [advisories].ignore but has no structured header block"
done
for id in "${BLOCK_IDS[@]}"; do
  contains "$id" "${IGNORED_IDS[@]}" || fail "$id: has a structured header block but is not in [advisories].ignore"
  for f in "${REQUIRED_FIELDS[@]}"; do
    [[ -n "${FIELD[$id|$f]:-}" ]] || fail "$id: missing '$f:' field in its structured header"
  done
done

# ── Check 2: no expired entry ────────────────────────────────────────────────
today_epoch=$(date +%s)
for id in "${BLOCK_IDS[@]}"; do
  exp="${FIELD[$id|expires]:-}"
  [[ -z "$exp" ]] && continue # already reported above
  # The format first: `date -d` also reads "tomorrow" or "+1 year", which move
  # with every run, so an entry carrying one would never expire.
  [[ "$exp" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] \
    || { fail "$id: expires '$exp' is not a YYYY-MM-DD date"; continue; }
  exp_epoch=$(date -d "$exp" +%s 2>/dev/null) || { fail "$id: expires '$exp' is not a valid YYYY-MM-DD date"; continue; }
  if (( exp_epoch < today_epoch )); then
    fail "$id: suppression expired on $exp — re-verify the reachability claim, then renew or remove it"
  fi
done

# ── Check 3: no stale suppression (raw cargo-audit outside repo scope) ─────
RAW_DIR="$(mktemp -d)"
trap 'rm -rf "$RAW_DIR"' EXIT
# What a raw audit raises, one "<id>\t<crate>\t<version>" line per affected
# package. Read from cargo-audit's JSON report rather than its text, so the
# package each advisory hits is kept: check 4 needs the versions in the graph
# today, not only the one the header names. AUDIT_RAW_JSON names a saved report
# to read instead of running cargo-audit; the self-test uses it.
raw_packages() {
  jq -r '[.vulnerabilities.list[]?, (.warnings // {} | to_entries[] | .value[]?)]
         | .[] | select(.advisory.id != null)
         | "\(.advisory.id)\t\(.package.name)\t\(.package.version)"' "$1"
}
RAW_PACKAGES=""
if [[ -n "${AUDIT_RAW_JSON:-}" ]]; then
  RAW_PACKAGES="$(raw_packages "$AUDIT_RAW_JSON")" || fail "could not read the raw audit report $AUDIT_RAW_JSON"
else
  for ws in "${WORKSPACES[@]}"; do
    cp "$ws/Cargo.lock" "$RAW_DIR/Cargo.lock"
    # The `cd` is what makes this run "raw": cargo-audit resolves .cargo/audit.toml
    # relative to the working directory, so from a scratch dir it reports the
    # unsuppressed set. `|| true` is scoped to the audit alone, not to `cd &&
    # audit` — cargo-audit exits non-zero whenever it finds anything, which is
    # expected here, but a failed `cd` must not be swallowed the same way: it would
    # leave the report empty and every suppression would read as stale.
    if ! (cd "$RAW_DIR" && { cargo audit --json --file Cargo.lock > report.json 2> /dev/null || true; }); then
      fail "could not enter scratch dir '$RAW_DIR' to run the unsuppressed audit"
      continue
    fi
    # A report that is not JSON means the audit did not run, which must not read
    # as "nothing fires" and so turn every suppression stale.
    if [[ ! -s "$RAW_DIR/report.json" ]] || ! out="$(raw_packages "$RAW_DIR/report.json" 2> /dev/null)"; then
      fail "the unsuppressed cargo-audit run on $ws/Cargo.lock produced no report"
      continue
    fi
    RAW_PACKAGES+="$out"$'\n'
  done
fi
# `|| true` because a clean raw audit is a legitimate outcome, not an error:
# grep exits 1 when it matches nothing, `set -o pipefail` promotes that to the
# whole pipeline, and `set -e` then killed this script with no message and no
# exit code anyone could act on. That is the exact moment every suppression has
# just become stale, so it has to report rather than die.
RAW_IDS="$(cut -f1 <<< "$RAW_PACKAGES" | grep -x 'RUSTSEC-[0-9]\{4\}-[0-9]\{4\}' | sort -u || true)"

for id in "${IGNORED_IDS[@]}"; do
  grep -qx "$id" <<< "$RAW_IDS" || fail "$id: does not appear in a raw (no .cargo/audit.toml) cargo-audit run — the advisory no longer fires; delete this entry, don't just leave it suppressed"
done

# Confirms `pkgspec` (a bare crate name, or `name@version` to disambiguate a
# crate with multiple resolved versions) is absent from the dependency graph
# under the given edge mode ($3: "all" for every edge, "normal" to exclude
# dev- and build-dependencies). `cargo tree` has two distinct "absent" shapes
# that both mean the same thing here and must both count as confirmed: exit 0
# with empty stdout (a bare name simply isn't in the graph), and a non-zero
# exit whose stderr says "did not match any packages" (a version-pinned spec
# naming a version that isn't resolved at all — the ambiguity that motivates
# pinning `name@version` in the first place, see git history). Any other
# non-zero exit (e.g. "is ambiguous", meaning the spec still matches more
# than one resolved version) is a real problem, not a pass. Calls fail()
# itself with a specific reason whenever it cannot confirm absence.
graph_absent_in() {
  local id="$1" pkgspec="$2" edge_mode="$3" build_label="$4"; shift 4
  local out err_file err
  # Built as an array rather than interpolated from a string: `cargo tree` needs
  # `-e normal` as two argv entries, and an unquoted "$edge_flag" relied on word
  # splitting to produce them (shellcheck SC2086).
  local -a edge=()
  [[ "$edge_mode" == "normal" ]] && edge=(-e normal)
  err_file="$(mktemp)"
  if out="$(cargo tree -i "$pkgspec" "${edge[@]}" --target all "$@" 2>"$err_file")"; then
    rm -f "$err_file"
    if [[ -n "$out" ]]; then
      fail "$id: '$pkgspec' resolves in the $build_label dependency graph"
      return 1
    fi
    return 0
  fi
  err="$(cat "$err_file")"
  rm -f "$err_file"
  if grep -q "did not match any packages" <<< "$err"; then
    return 0
  fi
  fail "$id: could not verify '$pkgspec' against the $build_label graph — $err"
  return 1
}

# A reachability claim has to hold for what a consumer can build, not just for
# `cargo build` with no flags: a crate absent by default but present behind a
# feature reaches every consumer who turns that feature on. So every absence
# claim is checked with every feature on, in each workspace the register covers.
graph_absent() {
  local id="$1" pkgspec="$2" edge_mode="$3" ok=0 ws
  for ws in "${WORKSPACES[@]}"; do
    graph_absent_in "$id" "$pkgspec" "$edge_mode" "$ws (all features)" \
      --manifest-path "$ws/Cargo.toml" --workspace --all-features || ok=1
  done
  return "$ok"
}

# ── Check 4: class claims hold against the dependency graph ────────────────
for id in "${BLOCK_IDS[@]}"; do
  class="${FIELD[$id|class]:-}"
  crate="${FIELD[$id|crate]:-}"
  # `crate:` is "<name> <version>" — use name@version so an `-i` lookup can't
  # land on the wrong one of two resolved versions of the same crate.
  crate_name="${crate%% *}"
  pkgspec="$crate_name"
  [[ "$crate" == *" "* ]] && pkgspec="${crate_name}@${crate#* }"
  anchor="${FIELD[$id|anchor]:-}"
  # The header names the version the entry was written for. The advisory may
  # since have moved to another version of the crate, or to another crate, and
  # a lookup of a version no longer resolved reads as absent. So an absence
  # claim is also checked for every package the raw audit says it hits today.
  specs=("$pkgspec")
  while IFS=$'\t' read -r _ name version; do
    [[ -n "$name" ]] && specs+=("$name@$version")
  done < <(awk -F'\t' -v id="$id" '$1 == id' <<< "$RAW_PACKAGES" | sort -u)

  case "$class" in
    not-in-graph)
      for spec in "${specs[@]}"; do graph_absent "$id" "$spec" "all" || true; done
      ;;
    dev-only|build-time-only)
      for spec in "${specs[@]}"; do graph_absent "$id" "$spec" "normal" || true; done
      ;;
    reachable-but-mitigated)
      # anchor is "path/to/file.rs::SYMBOL"
      anchor_path="${anchor%%::*}"
      anchor_symbol="${anchor##*::}"
      if [[ ! -f "$anchor_path" ]]; then
        fail "$id: anchor file '$anchor_path' does not exist"
      elif ! grep -q "$anchor_symbol" "$anchor_path"; then
        fail "$id: anchor symbol '$anchor_symbol' not found in '$anchor_path'"
      fi
      ;;
    reachable-accepted)
      : # manual sign-off; no automated check by design
      ;;
    *)
      fail "$id: unrecognised class '$class' (expected not-in-graph, dev-only, build-time-only, reachable-but-mitigated, or reachable-accepted)"
      ;;
  esac
done

# ── Check 5: nothing is ignored in deny.toml without an entry here ──────────
# Every RUSTSEC ID inside deny.toml's `[advisories] ignore`, whether written as
# a string or as `{ id = "...", reason = "..." }`.
DENY_IDS="$(awk '
  /^\[/ { in_adv = ($0 ~ /^\[[[:space:]]*advisories[[:space:]]*\][[:space:]]*(#.*)?$/); in_list = 0; next }
  in_adv && /^[[:space:]]*ignore[[:space:]]*=/ { in_list = 1 }
  in_list {
    rest = $0
    sub(/#.*/, "", rest)
    while (match(rest, /RUSTSEC-[0-9]{4}-[0-9]{4}/)) {
      print substr(rest, RSTART, RLENGTH)
      rest = substr(rest, RSTART + RLENGTH)
    }
    if ($0 ~ /\]/ && $0 !~ /\{/) in_list = 0
  }
' "$DENY_TOML" | sort -u)"
while IFS= read -r id; do
  [[ -n "$id" ]] || continue
  contains "$id" "${IGNORED_IDS[@]}" \
    || fail "$id: ignored in $DENY_TOML but not in $AUDIT_TOML, so it carries no structured header block"
done <<< "$DENY_IDS"

if [[ "$STATUS" -eq 0 ]]; then
  echo "check-audit-register: ${#BLOCK_IDS[@]} entries, all current."
fi
exit "$STATUS"
