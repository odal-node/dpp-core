# =============================================================================
# justfile — Odal Node (dpp-core) task runner
# Install: cargo install just
# Usage:   just <recipe>
# =============================================================================

# ---------------------------------------------------------------------------
# Quality gates
# ---------------------------------------------------------------------------

# Run all tests with nextest.
# --all-features is required, not cosmetic: no crate in this workspace turns on
# dpp-rules' `bundle` feature, so without it the signed-ruleset format and its
# fail-closed verification are never compiled or tested here at all.
test:
    cargo nextest run --workspace --all-features

# Run doctests, including each crate's README.
#
# Separate from `test` because `cargo nextest` does not run doctests at all —
# which is how every README example in this workspace went years without being
# compiled, several of them advertising functions that do not exist. Each
# publishable crate pulls its README in via a `ReadmeDoctests` item.
test-doc:
    cargo test --doc --workspace --all-features

# Run clippy (all warnings are errors). --all-features for the reason above.
lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format all code
fmt:
    cargo fmt --all

# Check formatting without modifying files (CI-safe)
fmt-check:
    cargo fmt --all --check

# Run security audit against RustSec advisory database
audit:
    cargo audit

# Build documentation (warns on missing docs)
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features

# Run Criterion benchmarks
bench:
    cargo bench --package dpp-benches

# Run each sector plugin's own test suite on the host target.
#
# The plugins are excluded from the workspace, so `just test` does not reach
# them: a plugin can be broken while the workspace gate is green. Runs on the
# host (not wasm32) because these are ordinary #[cfg(test)] unit tests; the
# wasm build is covered separately by `build-plugins`.
test-plugins:
    #!/usr/bin/env bash
    set -euo pipefail
    for plugin in plugins/sector-*; do
        [ -f "$plugin/Cargo.toml" ] || continue
        echo "Testing $plugin..."
        (cd "$plugin" && cargo test --quiet)
    done
    echo "All plugin tests passed."

# Run all gate checks (fmt → lint → test → plugin tests → doc → audit)
check: fmt-check lint test test-doc test-plugins doc audit

# `check` is a subset of CI: it never cross-compiles, so the two WASM jobs and
# the orphaned-tests guard can fail in CI on a change that passed locally. That
# is not hypothetical — a crate published as wasm32-safe can acquire a host-only
# dependency through an ordinary-looking call, and nothing in `check` notices.
#
# Deliberately a superset, not a mirror: `doc` and `test-doc` run here and not in
# CI, because a broken intra-doc link is cheaper to catch now than after a
# release. Keep in step with `.github/workflows/` when jobs change.

# Everything CI runs, plus docs. Run before pushing; `check` is the inner loop.
ci: check wasm-build build-plugins test-count

# `dpp-registry` and `dpp-domain` are consumed from WebAssembly, so neither may
# reach for a host-only facility (platform entropy, filesystem, threads). Built
# per-crate, not workspace-wide, because most of the workspace is *not*
# wasm32-safe by design and never needs to be.

# Cross-compile the crates that must stay wasm32-safe.
wasm-build:
    #!/usr/bin/env bash
    set -euo pipefail
    for crate in dpp-registry dpp-domain; do
        echo "Building $crate (wasm32-unknown-unknown)..."
        (cd "crates/$crate" && cargo build --target wasm32-unknown-unknown --release)
    done
    echo "wasm32 builds passed."

# `dpp-tests` exists for cross-crate integration coverage; if it stops being
# built, every one of those tests disappears and the suite still reports green.
# Counting them is the only signal that they are still there at all.

# Guard against `dpp-tests` silently detaching from the workspace.
test-count:
    #!/usr/bin/env bash
    set -euo pipefail
    COUNT=$(cargo test -p dpp-tests --all-features --tests -- --list 2>&1 | grep -c ': test' || echo 0)
    echo "dpp-tests integration tests found: $COUNT"
    if [ "$COUNT" -eq 0 ]; then
        echo "ERROR: zero integration tests — dpp-tests may be orphaned from the workspace"
        exit 1
    fi

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------

# Release build for all workspace crates
build:
    cargo build --workspace --release

# Build all Wasm sector plugins (requires wasm32-wasip1 target)
build-plugins:
    #!/usr/bin/env bash
    set -euo pipefail
    for plugin in \
        plugins/sector-battery \
        plugins/sector-textile \
        plugins/sector-steel \
        plugins/sector-electronics \
        plugins/sector-construction \
        plugins/sector-tyre \
        plugins/sector-toy \
        plugins/sector-aluminium \
        plugins/sector-furniture \
        plugins/sector-detergent; do
        echo "Building $plugin..."
        (cd "$plugin" && cargo build --target wasm32-wasip1 --release)
    done
    echo "All plugins built."

# Build a single sector plugin and copy it to the engine plugins dir.
# Usage: just build-plugin sector-battery   or just build-plugin battery
# Experimental Note: This is a temporary workaround until we have a proper plugin build system.
build-plugin PLUGIN:
    #!/usr/bin/env bash
    set -euo pipefail
    ROOT_DIR="$(pwd)"
    PLUGIN_RAW="{{PLUGIN}}"
    if [ -z "$PLUGIN_RAW" ]; then
        echo "Usage: just build-plugin sector-battery  (or just build-plugin battery)"
        exit 1
    fi
    # Normalize name: accept "sector-battery" or "battery"
    PLUGIN_NAME="${PLUGIN_RAW#sector-}"
    PLUGIN_DIR="${ROOT_DIR}/plugins/sector-${PLUGIN_NAME}"
    if [ ! -d "$PLUGIN_DIR" ]; then
        echo "Plugin directory not found: $PLUGIN_DIR"
        exit 2
    fi
    echo "Building $PLUGIN_DIR"
    (cd "$PLUGIN_DIR" && cargo build --target wasm32-wasip1 --release)
    # Copy artifact to sibling dpp-engine/plugins as sector-<name>.wasm.
    # All 10 plugins share one workspace (plugins/Cargo.toml), so the build
    # output lands in plugins/target, not plugins/sector-<name>/target.
    DEST_DIR="${ROOT_DIR}/../dpp-engine/plugins"
    mkdir -p "$DEST_DIR"
    ART="${ROOT_DIR}/plugins/target/wasm32-wasip1/release/sector_${PLUGIN_NAME}.wasm"
    cp "$ART" "${DEST_DIR}/sector-${PLUGIN_NAME}.wasm"
    echo "Copied $ART → ${DEST_DIR}/sector-${PLUGIN_NAME}.wasm"

# ---------------------------------------------------------------------------
# Cleanup
# ---------------------------------------------------------------------------

# Clean build artefacts
clean:
    cargo clean
