# Code Layout

Where code goes in this repository, and what enforces it.

This is a standard, not a plan. Every rule below is either **enforced** by a
tripwire in `crates/dpp-tests/tests/` or explicitly marked **guidance**. There is
no third category, on purpose — see §4.

---

## 1. The rules

### Rule 1 — one public type per file, when the type has gravity

A type has gravity when it has its own `impl` blocks, serde derives beyond a
plain derive line, or runs to roughly 40 lines including docs. Such a type gets
its own file, named for it in snake_case: `FacilitySnapshot` → `facility_snapshot.rs`.

A type and its own error enum are *one* concept and belong together. Three or
more public types in a file is the point at which the file has stopped being
about one thing.

### Rule 2 — `mod.rs` is a pure index

Module docs, `pub use` re-exports, and submodule declarations. Zero `pub struct`,
`pub enum`, `pub trait`, `pub fn`. This keeps every `mod.rs` skimmable and forces
a new type into its own named file as a module grows.

### Rule 3 — free functions group by verb-domain *(guidance)*

`validation/batch.rs`, not one file per three-line helper. **Not enforced**, and
deliberately so — see §4.

### Rule 4 — a `tests.rs` splits when it passes 400 lines

Along the same seams its source split. A test file nobody can navigate is a test
file nobody reads before changing the thing it covers.

### Rule 5 — an enum with `impl` gravity is a type under rule 1

State machines like `PassportStatus` count. A small closed enum that exists only
as one parent's field may ride along in the parent's file.

### Rule 6 — only `lib.rs` at `src/` root

Everything else lives in a directory module. `test_support.rs` is the single
permitted exception (rule 9).

### Rule 7 — tests live in a sibling file, never inline

`tests.rs` beside the module, or `golden_vectors.rs` where the module implements
a published methodology and the tests are its vectors. No `#[cfg(test)] mod tests {}`
blocks inside a source file.

The reason is rule 4: tests inline in a source file have no size of their own, so
nothing can tell you when they have outgrown it.

### Rule 8 — every file opens with a `//!` module doc

Within the first three lines. A file that cannot say what it is for in one line
is usually a file that holds two things.

### Rule 9 — shared test scaffolding is `test_support.rs`

One name, one place: the crate root, feature-gated. Not `fixtures.rs`, not
`helpers.rs`. This is the one file rule 6 allows beside `lib.rs`, because
scaffolding that is hard to find gets rewritten instead of reused.

### Rule 10 — integration tests are named for their kind

In `crates/dpp-tests/tests/`, a **tripwire** — a test that asserts a structural
or documentary invariant rather than behaviour — is prefixed `layout_` when it
guards this document, and otherwise named for the invariant it guards
(`domain_concerns`, `ports_inventory`, `mod_rs_is_pure_index`,
`open_product_group_lane`, `provisional_schema_marker`, `schema_conformity`).
Behavioural tests are named for the behaviour (`battery_end_to_end`,
`access_gatekeeping`, `transfer_of_responsibility`).

They fail for different reasons and are read by different people: a red tripwire
means the repo drifted from its own rules, a red behavioural test means the code
is wrong.

> **A note on why this is a naming convention and not a directory.** Cargo
> auto-discovers test binaries only from `.rs` files at the top level of
> `tests/`. Moving these into `tests/tripwires/` would require an explicit
> `[[test]]` entry per file in `Cargo.toml` — and a tripwire that silently does
> not run because someone forgot an entry is a worse failure than a flat
> directory. Auto-discovery is the safer property; the prefix does the grouping.

---

## 2. Deviations are legal, and counted

Any file that knowingly breaks an enforced rule carries a marker on its own line:

```rust
// LAYOUT-DEVIATION: <the reason, in one line>
```

The tripwire honours the marker and the file stops failing. It does **not** stop
being visible: the marker is greppable, and reviewing them periodically is how
the rules get revised rather than quietly abandoned.

A rule with no escape hatch is deleted the first time it is inconvenient. A rule
with a *counted* escape hatch survives.

---

## 3. What enforces what

All but rule 2 live in `crates/dpp-tests/tests/layout.rs`, one `#[test]` each,
sharing one directory walk. Rule 2 keeps its own file because it predates the
rest and works.

| Rule | Test | Fails when |
|---|---|---|
| 1, 5 | `layout::rule_1_one_public_type_per_file` | a source file declares ≥3 public types |
| 2 | `mod_rs_is_pure_index` | a `mod.rs` declares a public item |
| 3 | — | *guidance only* |
| 4 | `layout::rule_4_tests_files_are_navigable` | a `tests.rs` exceeds 400 lines |
| 6, 9 | `layout::rule_6_only_lib_rs_at_src_root` | a crate has a root `.rs` other than `lib.rs`, `main.rs` or `test_support.rs` |
| 7 | `layout::rule_7_tests_are_siblings_not_inline` | a source file contains an inline `#[cfg(test)] mod tests {` |
| 8 | `layout::rule_8_every_file_has_module_docs` | a `.rs` file has no `//!` in its first three lines |

The set of crates and plugins each one scans is **discovered from the directory
tree**, not listed. A hardcoded roster is how a new crate ends up silently
unchecked, which is the failure these tests exist to prevent.

**Rule 1's tripwire is a proxy, not the rule.** It fires at three public types
because a type plus its error is idiomatic and should not need a marker. Two
unrelated types in a file still break rule 1; the tripwire simply cannot tell
"related" from "unrelated" and does not pretend to.

### The baseline

Each tripwire carries an explicit, enumerated list of the files that already
violate it, in the test file itself. Those are allowed. **Anything not on the
list fails.** So the rules bind for all new and moved code from the day they
landed, while the existing backlog is worked through separately, and the list
shrinking is the visible measure of that work.

Entries are removed as files are fixed. **Never add to a baseline** to make a
build green — that is what the deviation marker is for, and unlike a baseline
entry a marker has to state a reason.

---

## 4. Why every rule is either enforced or labelled guidance

This standard existed before this document, in five numbered rules. Exactly one
of them had a test. That rule had **zero** violations. Every other rule had
many — twelve oversized test files, one file with twelve public types, twenty
eight files with no module doc, sixty one inline test modules.

The rules were not wrong and nobody ignored them on purpose. They simply had
nothing watching them, and a rule with nothing watching it is a preference.
Preferences lose to deadlines.

Rule 3 stays guidance because it cannot be tested without a definition of
"verb-domain" that nobody would agree on. That is a reason to label it honestly,
not a reason to promote it — an unenforceable rule sitting in a list of enforced
ones is what teaches a reader that the list is decorative.

**A tripwire is not trusted until it has been seen to fail.** Introduce a
violation, watch it go red, revert. A gate nobody has watched fail is a gate
nobody knows is wired up.

---

## 5. Scope

These rules govern `crates/*/src/`, `plugins/*/src/`, and
`crates/dpp-tests/tests/`. They are invisible to consumers: every move is
internal, with `pub use` in `lib.rs` preserving public paths. A layout change
that alters the public API is not a layout change.
