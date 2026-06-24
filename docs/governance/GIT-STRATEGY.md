# Git Strategy — dpp-core

**Model:** Trunk-based development
**Remote:** `git@github.com:odal-node/dpp-core.git`
**Primary branch:** `main`

---

## Branch Structure

```
main                    ← always deployable, tagged releases
  └── feat/*            ← feature branches (short-lived, <1 week)
  └── fix/*             ← bug fixes
  └── chore/*           ← CI, docs, formatting, dependency bumps
  └── release/v*        ← optional: release prep branch if needed
```

No `develop` branch. No long-lived branches. Everything merges to `main` via PR.

---

## Recommended Initial Commit Sequence

When you're ready to push, structure the history into clean logical commits:

```
1. feat: foundation — workspace, Cargo.toml, schemas, justfile, CI
2. feat(dpp-domain): domain types, passport, sector data, port traits
3. feat(dpp-domain): transfer of responsibility model
4. feat(dpp-domain): versioned schema registry with hot-reload
5. feat(dpp-crypto): Ed25519 key management, JWS, did:web builder
6. feat(dpp-crypto): verifiable credentials and access policy engine
7. feat(dpp-digital-link): digital link parser, link-type negotiation, AAS mapping
8. feat(dpp-registry): EU Central Registry interface types
9. feat(dpp-plugin-traits): wasm plugin ABI with capability negotiation
10. test: integration tests (textile e2e, transfer, access tier, schema conformity)
11. docs: architecture docs, conformity statement, README
```

Or if you prefer fewer commits:

```
1. Initial commit — workspace foundation, schemas, CI
2. feat: core crates (domain, rules, crypto, digital-link, calc, plugin-traits, plugin-sdk, registry)
3. test: unit and integration tests
4. docs: architecture, conformity, README
```

---

## Commit Convention

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

Types: feat, fix, test, docs, chore, refactor, ci
Scopes: dpp-domain, dpp-crypto, dpp-digital-link, dpp-rules, dpp-calc, dpp-plugin-traits, dpp-plugin-sdk, dpp-registry, dpp-tests
```

Examples:
```
feat(dpp-domain): add textile SVHC substance disclosure fields
fix(dpp-digital-link): add lifetime annotation to negotiate function
test(dpp-crypto): access tier gatekeeping integration tests
docs: update README with regulatory coverage table
chore: bump chrono to 0.4.39
ci: split unit and integration test steps
```

---

## Release Tagging

```
v0.1.0    ← first public release (current state)
v0.1.1    ← patch: bug fixes, formatting
v0.2.0    ← minor: new sector schemas, additional port traits
v1.0.0    ← major: stable API after textile delegated act is finalised
```

Tag format: `v{MAJOR}.{MINOR}.{PATCH}`

Create tags on `main` only:
```bash
git tag -a v0.1.0 -m "Initial public release — core DPP library"
git push origin v0.1.0
```

---

## PR Workflow

1. Create a feature branch: `git checkout -b feat/battery-soh-tracking`
2. Make changes, commit with conventional format
3. Push and open PR against `main`
4. CI runs: `fmt-check → clippy → test → audit`
5. Squash-merge to `main` (keeps history clean)
6. Delete the feature branch

---

## Branch Protection (set up on GitHub)

For `main`:
- Require PR reviews (1 reviewer minimum)
- Require CI status checks to pass
- Require linear history (squash merges)
- No force pushes
- No direct commits (everything via PR)

---

## First Push Checklist

```bash
# 1. Make sure everything passes
just fmt
just check

# 2. Create the GitHub repo (via CLI or web UI)
gh repo create odal-node/dpp-core --private --description "EU Digital Product Passport Standard Library"

# 3. Set remote and push
cd dpp-core
git remote add origin git@github.com:odal-node/dpp-core.git
git branch -M main
git push -u origin main

# 4. Tag the initial release
git tag -a v0.1.0 -m "Initial release — core DPP library"
git push origin v0.1.0

# 5. Set branch protection rules on GitHub
# Settings → Branches → Add rule for 'main'

# 6. When ready to go public
gh repo edit odal-node/dpp-core --visibility public
```

---

## Visibility Strategy

| Phase | Visibility | Reason |
|---|---|---|
| **Now** | Private | Polish, fix remaining clippy/fmt issues, verify CI green |
| **After CI green** | Private | Tag v0.1.0, review README one more time |
| **When ready** | Public | Announce on LinkedIn, submit to GS1 Solution Partner |

Start private. Going public is a one-way door for first impressions. Make sure `just check` is fully green, the README renders well on GitHub, and the license file is correct before flipping the switch.

---

## .gitignore Additions

Make sure these are in `.gitignore` before pushing:

```
/target/
*.swp
*.swo
.DS_Store
.idea/
.vscode/
```

---

## Crates.io Publishing (Future)

When the API stabilises (post-v1.0.0):

```bash
cargo publish -p dpp-rules          # no internal deps
cargo publish -p dpp-plugin-traits  # no internal deps
cargo publish -p dpp-domain         # depends on dpp-rules
cargo publish -p dpp-crypto         # depends on dpp-domain
cargo publish -p dpp-digital-link   # depends on dpp-domain
cargo publish -p dpp-registry       # depends on dpp-domain
cargo publish -p dpp-calc           # depends on dpp-domain
cargo publish -p dpp-plugin-sdk     # depends on dpp-plugin-traits + dpp-rules
```

Publish order matters — dependencies first. `dpp-tests` is `publish = false` and is never published. Sector plugins are released as `.wasm` artefacts, not to crates.io.
