# Code Layout

Where code goes in this repository, and what enforces it.

This is a standard, not a plan. Every rule below is either **enforced** by a
tripwire in `crates/dpp-tests/tests/` or explicitly marked **guidance**. There is
no third category, on purpose — see §5.

§1 says which module a thing belongs in. §2 says how that module is arranged
internally. They answer different questions and a file has to satisfy both.

---

## 1. The scope law

Every module sits in exactly one of four tiers. A module's tier is a property of
**what it does**, not of what it contains, and **imports may only point up the
ladder**.

| Tier | Name | Does | May import |
|---|---|---|---|
| 1 | vocabulary | Names things. Decides nothing. | nothing in this crate |
| 2 | model | Records what is true. | tier 1, and tier 2 acyclically |
| 3 | policy | Decides something. | tiers 1–2 |
| 4 | ports | States what the outside must provide. | tiers 1–3 |

**Nothing may import tier 4.** Adapters live in the platform repository and are
already outside this crate; every `impl …Port for` in the workspace is there.

`error/` sits **above** the ladder rather than inside it. It is the one module
that may name a type from any tier, because a crate-wide error has to be able to
carry any of them.

### Why a ladder and not a diagram

The tiers exist to make one question answerable without argument: *when two
modules refer to each other, which direction is the mistake?* Undirected cycles
cannot answer it — every cycle has a wrong edge and a right one, and with no law
you can only report that a cycle exists. That is why an earlier review of this
crate concluded a module split was unavailable: it measured cycles without a rule
for which way they should have run.

With the ladder, the wrong direction is named and the count is small. Measured
against `main` on 2026-08-25: 129 production import edges between top-level
modules, **five illegal**, four of them on one type.

### Rule 0 — a type lives under `ports/<x>/` only if that port is its sole consumer

This is the test for whether something is really tier 4.

`ArchiveReceipt` passes: nothing but `ArchivePort` touches it, so it belongs
beside the trait. `SealedEnvelope` and `ComplianceResult` fail — both are fields
on `Passport`, so filing them under `ports/` puts a tier-2 aggregate in the
position of importing tier 4. They are model values that were misfiled, and
moving the *types* fixes it without touching the aggregate.

**An aggregate keeps its own invariants.** Enforcing them is what an aggregate is
for, and moving that logic out to satisfy a layer diagram produces an anaemic
model — a worse fault than the one being fixed, and an invisible one, because
everything still compiles. What an aggregate may not do is depend on a *service*:
a registry, a repository, a client. So the cut is at the dependency, never at the
method.

**Factories are exempt.** An associated function that constructs the type may
consult a tier-3 service — `Passport::from_stored(doc, &LensRegistry, &Catalog)`
applies schema lenses to rehydrate a stored record, and that is construction, not
behaviour. The exemption is written down because a naive tripwire would flag it.

---

## 2. The rules

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
deliberately so — see §5.

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

### Rule 10 — integration tests are named for their kind *(guidance)*

In `crates/dpp-tests/tests/`, a **tripwire** — a test that asserts a structural
or documentary invariant rather than behaviour — is named for the invariant it
guards: `layout` for this document, and otherwise `domain_concerns`,
`ports_inventory`, `mod_rs_is_pure_index`, `open_product_group_lane`,
`provisional_schema_marker`, `schema_conformity`. Behavioural tests are named
for the behaviour (`battery_end_to_end`, `access_gatekeeping`,
`transfer_of_responsibility`).

They fail for different reasons and are read by different people: a red tripwire
means the repo drifted from its own rules, a red behavioural test means the code
is wrong.

> **A note on why this is a naming convention and not a directory.** Cargo
> auto-discovers test binaries only from `.rs` files at the top level of
> `tests/`. Moving these into `tests/tripwires/` would require an explicit
> `[[test]]` entry per file in `Cargo.toml` — and a tripwire that silently does
> not run because someone forgot an entry is a worse failure than a flat
> directory. Auto-discovery is the safer property; the name does the grouping.

### Rule 11 — one file for a concept; two files means a directory

A concept that fits in one file is a file. The moment it needs a second — an
error type, a test file, a helper — it becomes a directory:

```
<concept>/
├── mod.rs        pure index (rule 2)
├── error.rs      its errors (rule 14), present iff it has any
├── tests.rs      or tests/ once it passes 400 lines (rule 4)
└── <part>.rs     one file per public type with gravity (rule 1)
```

**The rule then applies to that directory too, at every depth.** `identifier/` is
not a folder of loose files: `gtin/`, `gln/` and `commodity_code/` are each their
own module in this shape, and `gtin/` holds `check_digit.rs` inside it.

This is the shape `regex-automata/src/meta/` and `tokio/src/sync/` both use, and
it is what makes a module skimmable without opening it: the same four names
appear in every directory, so anything else in the listing is the actual subject.

### Rule 12 — a file never repeats the name of its directory

The path is already the namespace. `battery/data.rs` reads `battery::data` at
every call site; `battery/battery_data.rs` reads `battery::battery_data` and says
the word twice forever.

Applies to directories too: `ghosts/archive.rs`, not `ghosts/ghost_archive.rs`.

### Rule 13 — hyphens outside module paths, underscores inside

Crate directories, plugin directories and data directories take `-`
(`crates/dpp-domain/`, `product-groups/`, `schemas/unsold-goods/`), as do data
files (`unsold-goods.json`). Module directories and every `.rs` file take `_`.

The split is not taste. A directory under `src/` **is** a Rust identifier, and
identifiers cannot contain a hyphen — `mod product-group;` does not parse.
Reaching a hyphenated directory needs `#[path = "…"]` on every module, which
breaks editor navigation for no gain.

So the same concept is correctly spelled two ways, and both are already in the
tree: `dpp-domain/product-groups/` is data on disk, `dpp-aas/src/product_groups/`
is a module path.

### Rule 14 — a module's errors live in its `error.rs`

Not in the type's own file, and not in a crate-wide error bucket. A module's
failure modes are part of its contract and should be readable in one file without
reading the module.

The single exception is the crate-wide `error/`, which sits above the tiers (§1)
and carries `DppError` and the field-error types.

---

## 3. Deviations are legal, and counted

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

## 4. What enforces what

Every enforced rule but rule 2 lives in `crates/dpp-tests/tests/layout.rs`, one
`#[test]` each, sharing one directory walk. Rule 2 keeps its own file because it
predates the rest and works.

| Rule | Test | Fails when |
|---|---|---|
| 1, 5 | `layout::rule_1_one_public_type_per_file` | a source file declares ≥3 public types |
| 2 | `mod_rs_is_pure_index` | a `mod.rs` declares a public item |
| 3 | — | *guidance only* |
| 4 | `layout::rule_4_tests_files_are_navigable` | a `tests.rs` exceeds 400 lines |
| 6, 9 | `layout::rule_6_only_lib_rs_at_src_root` | a crate has a root `.rs` other than `lib.rs`, `main.rs` or `test_support.rs` |
| 7 | `layout::rule_7_tests_are_siblings_not_inline` | a source file contains an inline `#[cfg(test)] mod tests {` |
| 8 | `layout::rule_8_every_file_has_module_docs` | a `.rs` file has no `//!` in its first three lines |
| 10 | — | *guidance only* |
| 0 | `layout::tier_imports_point_up` | a module imports a higher tier |
| 11 | `layout::rule_11_an_outgrown_concept_is_a_directory` | two files in one directory share a stem prefix (`gtin.rs` beside `gtin_check_digit.rs`) |
| 12 | `layout::rule_12_no_name_repeats_its_directory` | a filename begins with its parent directory's name |
| 13 | `layout::rule_13_hyphens_outside_module_paths` | a `-` appears under `src/`, or a `_` in a crate or data directory |
| 14 | `layout::rule_14_errors_live_in_error_rs` | a `pub enum …Error` is declared outside an `error.rs` |

The set of crates and plugins each one scans is **discovered from the directory
tree**, not listed. A hardcoded roster is how a new crate ends up silently
unchecked, which is the failure these tests exist to prevent.

> **Rules 0 and 11–14 are new in this pull request and their tests land in the
> next commit on the same branch.** Until they do, this document names five gates
> that do not exist, which is exactly the third category §5 says must not exist.
> **This branch does not merge in that state** — the rules and their tripwires
> arrive together or the rules come back out.
>
> Rule 11's gate is a proxy, in the same sense rule 1's is. A test cannot tell
> whether two files are one concept; it can tell that `gtin.rs` sits beside
> `gtin_check_digit.rs`, which is what an outgrown concept looks like from the
> outside. The rule is larger than the gate and the document says so rather than
> pretending otherwise.

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

## 5. Why every rule is either enforced or labelled guidance

This standard existed before this document, in five numbered rules. Exactly one
of them had a test. That rule had **zero** violations. Every other rule had
many — twelve oversized test files, two files with twelve public types, twenty
six files with no module doc, seventy one inline test modules.

The rules were not wrong and nobody ignored them on purpose. They simply had
nothing watching them, and a rule with nothing watching it is a preference.
Preferences lose to deadlines.

Rule 3 stays guidance because it cannot be tested without a definition of
"verb-domain" that nobody would agree on. Rule 10 stays guidance for the same
kind of reason: a test could assert that every file in `tests/` is on a known
list, but the list would have to be hand-maintained, so it would catch a
*rename* and miss the thing that actually matters — a tripwire named as though
it were behavioural. That is a reason to label both honestly, not a reason to
promote them — an unenforceable rule sitting in a list of enforced ones is what
teaches a reader that the list is decorative.

**A tripwire is not trusted until it has been seen to fail.** Introduce a
violation, watch it go red, revert. A gate nobody has watched fail is a gate
nobody knows is wired up.

---

## 6. Scope

§2's rules govern `crates/*/src/`, `plugins/*/src/`, and
`crates/dpp-tests/tests/`. §1's scope law governs the same trees. Rule 13 reaches
wider than either, because it is about directory names rather than code: it also
governs `crates/`, `plugins/`, and the data directories beside them.

### Most layout changes are invisible. The scope law is not.

A move within a module is internal, and `pub use` in `lib.rs` keeps the public
path steady. That holds for rules 1–14 and it is the normal case.

**§1 is the exception, and deliberately so.** Placing a module in its tier can
mean it stops being where it was — dissolving a wrapper module renames every path
beneath it, and `dpp-engine` reaches 98 distinct paths into this crate, most of
them deep. Those break.

That is a real cost and it is accepted rather than hidden. Compatibility
re-exports would keep it quiet, and they are refused on purpose: two ways to
reach the same type is the condition this standard exists to end, and a shim
outlives its migration by years. The crate has no external users, so the break is
paid once, mechanically, with the consumer updated in the same change.

**A tier move is therefore a breaking change and is versioned as one.** A layout
change that alters the public API for any *other* reason is still not a layout
change.
