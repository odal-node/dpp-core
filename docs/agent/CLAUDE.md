# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

---

**These guidelines are working if:** fewer unnecessary changes in diffs, fewer rewrites due to overcomplication, and clarifying questions come before implementation rather than after mistakes.

## Git Commit Rules

1. Keep commit titles under 50 characters, using imperative tense (e.g., "add fix" not "added fix")
2. Use Conventional Commits format: `<type>(scope): <subject>`
   - feat: new feature
   - fix: bug fix
   - docs: documentation
   - refactor: code change that doesn't fix bugs or add features
   - chore: build/tooling changes
3. NEVER include `Co-Authored-By` or any AI attribution tags in commit messages
4. NEVER commit or push code without approval

## Overview

**dpp-core** is the pure, stateless core library for the Odal Node Digital Product Passport system. It contains domain types, cryptographic primitives, schema validation, and port traits — all publishable under Apache-2.0. No database, no HTTP framework, no infrastructure dependencies.

**The Golden Rule**: If code changes because an EU regulation changed → it belongs here.

**The Compilation Test**: `cargo build --workspace` must succeed with zero infrastructure running — no DB, no Redis, no env vars.

## Crate Layout

```
dpp-domain        — domain types, port traits, VersionedSchemaRegistry, JSON Schema validation
dpp-crypto        — Ed25519 key management, AES-GCM, JWS sign/verify, did:web DID builder, LocalIdentityService
dpp-digital-link  — GS1 Digital Link parser (pure, no I/O)
dpp-plugin-traits — Wasm plugin host/guest contract: DppSectorPlugin trait, capabilities, AbiResult
dpp-plugin-sdk    — guest-side SDK: export_plugin! macro (generates the ABI incl. describe()) + Validator
dpp-registry      — EU Central Registry interface types (wasm32-safe)
dpp-rules         — pure no_std, zero-dep cross-field regulatory rules; shared by dpp-domain and the Wasm plugins (kept separate by design — target per-sector structure in docs/architecture/SECTOR-MODEL-CONSOLIDATION.md §7)
dpp-calc          — EU-methodology calculators (CO2e cradle-to-gate, EN 45554 repairability); pure, stateless; licensed LCI data injected via FactorProvider, never bundled
dpp-tests         — cross-crate integration tests (textile E2E, transfer of responsibility, access tiers, schema conformity)
```

Sector plugins (`plugins/sector-*`) are standalone Rust crates compiled to `wasm32-wasip1`, excluded from the workspace. Each implements `DppSectorPlugin` and calls `export_plugin!` once; **`sector-battery` is the reference implementation**. The host calls a plugin's `describe()` export and runs `check_compatibility` before dispatch. See `docs/architecture/PLUGIN-HOST.md`.

## Build and Development Commands

```sh
just check          # Full gate: fmt-check → lint → test → audit
just build          # Release build for all workspace crates
just build-plugins  # Compile Wasm sector plugins (wasm32-wasip1)
just test           # cargo nextest run --workspace
just lint           # cargo clippy --workspace --all-targets -- -D warnings
just fmt            # cargo fmt --all
just clean          # cargo clean
```

## Architecture

### Port Traits (dpp-domain::ports)

Port traits define the core/platform boundary:
- `PassportRepository` (async, persistence)
- `ComplianceRegistry` + `ComplianceStrategy` (non-async, sector dispatch)
- `IdentityPort` (async, sign/verify)
- `PluginHost` (non-async, Wasm dispatch)
- `ArchivePort` (async, immutable archival with retention guarantees)
- `RegistrySyncPort` (async, EU Central Registry registration/status sync)
- `SealPort` (async, eIDAS qualified electronic seal — ESPR Art. 13 / eIDAS 910/2014)

All implementations live in the platform repo.

### Schemas

Versioned JSON schemas at `crates/dpp-domain/schemas/{sector}/v{version}.json` (inside the crate so they ship on publish). The `VersionedSchemaRegistry` in dpp-domain embeds them via `include_str!()` and validates passport data at runtime. Adding a new schema version is a single file addition. **Never** `include_str!` a path outside the crate dir — `cargo publish` excludes it and the crate fails to build for downstream consumers.

### Wasm Targets

- `wasm32-unknown-unknown` — `dpp-registry` (EU registry types) and `dpp-digital-link`
- `wasm32-wasip1` — sector plugins (wasmtime host in platform)
Behavioral guidelines to reduce common LLM coding mistakes. Merge with project-specific instructions as needed.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.