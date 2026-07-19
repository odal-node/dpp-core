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

# Run all gate checks (fmt → lint → test → doc → audit)
check: fmt-check lint test doc audit

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
    # Copy artifact to sibling dpp-engine/plugins as sector-<name>.wasm
    DEST_DIR="${ROOT_DIR}/../dpp-engine/plugins"
    mkdir -p "$DEST_DIR"
    ART="$(ls "${PLUGIN_DIR}/target/wasm32-wasip1/release/"*.wasm | head -n1)"
    cp "$ART" "${DEST_DIR}/sector-${PLUGIN_NAME}.wasm"
    echo "Copied $ART → ${DEST_DIR}/sector-${PLUGIN_NAME}.wasm"

# ---------------------------------------------------------------------------
# Cleanup
# ---------------------------------------------------------------------------

# Clean build artefacts
clean:
    cargo clean
