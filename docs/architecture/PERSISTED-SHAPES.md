# Persisted shapes: changing a struct that is already on disk

**Status:** standard. Governs `Passport` and every `ProductGroupData` variant.
**Enforced by:** `crates/dpp-domain/tests/schema_compat.rs`, for one of the two halves
below. The other half has no automated check; §6 says so.

Most breaking changes in this workspace are free. Nothing is 1.0, the versioning policy
says a minor bump may break, and a consumer that stops compiling has been told. This rule
exists because two of the types do not work that way.

---

## 1. Why these types are different

`Passport` and the `ProductGroupData` variants are the literal on-disk shape of every
passport ever stored. A non-additive change to one of them does not break a consumer at
compile time. It makes every already-written document of that shape fail to deserialise
the moment a deployment upgrades its `dpp-domain` version: a **runtime failure against
data, discovered per request, with no compile-time signal**.

This has happened, and the cost is recorded:

- `TextileData.gtin` became required and `countryOfManufacturing` was renamed to
  `countryOfOrigin`. Downstream, that broke reads for **244 of 276 stored passports** on
  upgrade.
- `sector` to `productGroup` and `sectorData` to `productGroupData` renamed envelope keys.
  `product_group` is required, so every document written before the rename stopped
  deserialising, and the list endpoint returned 500 for any page containing one.

Both changes were correct. The failure was making them the way an ordinary breaking
change is made.

**So the usual reasoning does not transfer.** "Pre-1.0 breaking changes are free" is a
statement about *consumers who compile against us*. It says nothing about documents
already written, which do not recompile.

---

## 2. What is covered, and the split that matters

Two halves, with **different rules and different enforcement**. Conflating them is the
mistake this section exists to prevent.

| | Envelope | `productGroupData` |
|---|---|---|
| What | every field on `Passport` outside `product_group_data` | the per-product-group sub-object |
| Versioned? | **no**: there is no envelope schema version | yes, by `schema_version` |
| Lens escape hatch? | **never** | yes |
| Frozen-fixture coverage? | **none** (§6) | one per declared version |

`schema_version` applies to `product_group_data` **only**. There is no envelope version,
and none is planned.

**The envelope is never lensed, on purpose.** A lens transforms one product group's
sub-object. An envelope field is shared by every product group's stored documents, so a
faulty envelope transform would corrupt all of them at once. `Passport::from_stored` says
so at the method and `Passport::schema_version` at the field.

The consequence people get wrong: **the envelope has no escape hatch, so for the envelope,
"additive only" is the entire protection.**

---

## 3. What additive means

A change is additive when a document written before it still deserialises after it,
unchanged.

**Allowed:**

- A new field that is `Option<T>` with `#[serde(default)]`, or has a `#[serde(default)]`
  producing a meaningful value.
- A new enum variant on a type that only we ever *write*.
- A rename **that keeps accepting the old key**, through `#[serde(alias = "...")]` or a
  hand-written `Deserialize`.

**Not additive:**

- Making an existing optional field required.
- Renaming a key without keeping the old one readable.
- Changing a field's type, including the element type of a collection.
- Removing a field that a document may carry. §6 explains why this one is especially
  quiet.

`#[serde(default)]` protects a key that is **absent**. It does nothing for a key that is
*present* in a shape the type no longer accepts, which is why a changed element type fails
where an added field does not.

---

## 4. The escape hatch, and its exact reach

A non-additive change to `product_group_data` is allowed when it ships with:

1. a new schema version under `crates/dpp-domain/schemas/<product-group>/`,
2. a lens bridging the previous version to it, and
3. a frozen fixture for the new version.

`Passport::from_stored` then upcasts a stored document through the registered lens chain
before deserialising it.

**A lens may refuse.** Battery documents before `v2.5.0` are a documented refusal: `v2.5.0`
made `batteryType` required, because Annex VI Part A point 2 makes the battery category
mandatory public content, and no lens can invent a category for a record that predates the
requirement. Refusing is correct; inventing a value would be a fabricated regulatory claim.

**None of this reaches the envelope.** There is no envelope lens, so an envelope change
has exactly one allowed form: additive, per §3.

---

## 5. The second surface: stored is not the only one

A field in the **signed public view** is on disk *and* on the wire between operators, and
the two surfaces need opposite failure behaviour.

| | Stored document | Fetched document |
|---|---|---|
| Whose bytes | this deployment's own database | another operator's deployment |
| Signed? | yes | yes |
| Can it be rewritten? | **yes**, by its owner | **never, by anyone** |
| Right failure mode | **loud**: name a problem its owner can fix | **tolerant**: it must keep parsing indefinitely |

For a stored document, refusing to read is a good failure: it is visible, and the owner of
the bytes can migrate them.

For a fetched one it is not. A passport fetched from another operator is signed and
belongs to them. It cannot be migrated into a new shape, not by us, and not by them
without breaking their own signature. **A reader that refuses it refuses data that is
correct, current and unforgeable, permanently.**

The refusal is not quiet either. The verification walk that reads a fetched passport's
component references reports anything it cannot parse as a malformed reference, and the
evidence dossier grades a malformed reference as an integrity violation, alongside a hash
mismatch and a cycle. A deployment that upgraded would therefore report one that had not
as having a **tampered** bill of materials, on evidence that is only a version difference.
Deployments are independent and per operator, so that version skew is the normal steady
state, not a migration window.

**Rule:** for any field in the signed public view, decide compatibility from the *fetched*
column. It is strictly stricter than the stored one, and the stored analysis alone has
already given the wrong answer.

---

## 6. What is enforced, and what is not

`schema_compat.rs` holds one frozen document per `(product group, schema version)` and
asserts that each still reads through `from_stored`. A fixture is written once and never
regenerated, because, in the file's own words, *a fixture regenerated from the current
schema would agree with the current schema by construction and could not catch anything.*

That is a real check, and it covers `productGroupData`.

**It gives the envelope no coverage at all.** The fixtures are `productGroupData`-shaped;
the surrounding envelope is built by `stored_passport` as a **JSON literal in the current
shape**. So the envelope half of every fixture is exactly what the file warns against,
regenerated to agree with the present, and a renamed envelope key passes the test
unnoticed.

That is why both envelope renames in §1 shipped. It is not an argument for weakening the
rule: it is why the envelope rule has to be followed by hand, and why a reviewer should
treat any diff touching `Passport`'s fields or `PASSPORT_WIRE_KEYS` as needing an explicit
compatibility argument rather than a green test run.

Closing the gap means a frozen *envelope* fixture, independent of product group. Worth
doing; not done.

**One envelope failure is refused at read time, but only once it is recorded by hand.**
`Passport` does not set `deny_unknown_fields`, so a stored document carrying a key this
build removed deserialises *successfully*, with the replacement field at its default.
`parentPassportRef` to `derivedFrom` did exactly that: a second-life passport loaded with
no error and no predecessor links. `REMOVED_ENVELOPE_KEYS` in
`crates/dpp-domain/src/passport/record.rs` lists each removed key with its replacement,
and `Passport::from_stored` refuses any document carrying one with
`DppError::RemovedEnvelopeKey`. A direct `serde_json::from_value::<Passport>` bypasses that
check and still drops the key silently.

`removed_envelope_keys_name_real_replacements` holds the list to `PASSPORT_WIRE_KEYS`.
Nothing checks that a rename was added to it in the first place, so forgetting is silent.

---

## 7. If you have to break it anyway

Sometimes the right change is non-additive and has no lens, most obviously an envelope
rename. The rule then is not "never" but **"say so, and say what it costs"**:

- State it in `CHANGELOG.md` under `### Breaking`, with the migration a holder of existing
  documents must run.
- Say which failure mode it has: refusing to load, or loading with the field silently
  missing. The second is worse and must never go unstated.
- Close the silent one: add the old key and its replacement to `REMOVED_ENVELOPE_KEYS`, so
  `Passport::from_stored` refuses a document still carrying it and names the replacement
  (§6). Entries are permanent: a key removed two renames ago is still a key some stored
  document may carry.
- Record why it was acceptable *now*. Every such change so far rested on there being no
  published passports to strand. **That stops being true at the first real one**, and
  after that an envelope rename must keep the old key, or ship a one-time document rewrite
  in the publish pipeline.
- A rewritten document no longer verifies against a signature that covers the old key
  names. No envelope rename leaves a signed, published passport intact without a migration
  written for it.

This rule is cheap to follow now and impossible to retrofit later, which is why it is
written down before anyone has been hurt by it in production.
