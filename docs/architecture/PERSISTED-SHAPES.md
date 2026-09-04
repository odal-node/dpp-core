# Persisted Shapes — changing a struct that is already on disk

**Status:** standard. Governs `Passport` and every `ProductGroupData` variant.
**Enforced by:** `crates/dpp-domain/tests/schema_compat.rs` — for one of the two
halves below. The other half has no tripwire; §6 says so plainly.

Most breaking changes in this workspace are free. Nothing is 1.0, the versioning
policy says a minor bump may break, and a consumer that stops compiling has been
told. This rule exists because two of our types do not work that way.

---

## 1. Why these types are different

`Passport` and the `ProductGroupData` variants are the literal on-disk shape of
every passport a node has ever stored. A non-additive change to one of them does
not break a consumer at compile time. It makes every already-written document of
that shape undeserialisable the moment a node upgrades its `dpp-domain` pin — a
**runtime failure against data, discovered per request, with no compile-time
signal anywhere**.

That is not a hypothetical, and the cost is recorded:

- `TextileData.gtin` became required and `countryOfManufacturing` was renamed to
  `countryOfOrigin`. Downstream that took out reads for **244 of 276 passports**
  the instant the node upgraded.
- `sector` → `productGroup` and `sectorData` → `productGroupData` renamed
  envelope keys. `product_group` is required, so every pre-rename document
  stopped deserialising and the list endpoint returned 500 for any page
  containing one.

Both were correct changes. Neither was wrong to make. The failure was making them
the way an ordinary breaking change is made.

**So the usual reasoning does not transfer.** "Pre-1.0 breaking changes are free"
is a statement about *consumers who compile against us*. It says nothing about
documents already written, which do not recompile and cannot be asked to.

---

## 2. What is covered, and the split that matters

Two halves, with **different rules and different enforcement**. Conflating them
is the mistake this section exists to prevent.

| | Envelope | `productGroupData` |
|---|---|---|
| What | every field on `Passport` outside `product_group_data` | the per-product-group sub-object |
| Versioned? | **no** — there is no envelope schema version | yes, by `schema_version` |
| Lens escape hatch? | **never** | yes |
| Frozen-fixture coverage? | **none** (§6) | one per declared version |

`schema_version` is scoped to `product_group_data` **only**. There is no
equivalent version for the envelope, and no plan to add one.

**The envelope is never lensed, deliberately.** A lens transforms one product
group's sub-object. An envelope field is shared by every product group's stored
documents, so an envelope transform gone wrong corrupts all of them at once
rather than one. `Passport::from_stored` states this at the method and
`Passport::schema_version` states it at the field.

The consequence is the part people get wrong: **the envelope has no escape
hatch, so for the envelope "additive-only" is not a preference — it is the whole
of the protection.**

---

## 3. What additive means

A change is additive when a document written before it still deserialises after
it, unchanged.

**Allowed:**

- A new field that is `Option<T>` with `#[serde(default)]`, or has a
  `#[serde(default)]` producing a meaningful value.
- A new enum variant on a type that is only ever *written* by us.
- A rename **that keeps accepting the old key**, via `#[serde(alias = "...")]`
  or a hand-written `Deserialize`.

**Not additive:**

- Making an existing optional field required.
- Renaming a key without keeping the old one readable.
- Changing a field's type, including the element type of a collection.
- Removing a field that a document may carry — see §6 on why this one is
  particularly quiet.

`#[serde(default)]` protects a key that is **absent**. It does nothing for a key
that is *present* in a shape the type no longer accepts, which is why a changed
element type fails where an added field does not.

---

## 4. The escape hatch, and its exact reach

A non-additive change to `product_group_data` is allowed when it ships with:

1. a new schema version under `crates/dpp-domain/schemas/<product-group>/`, and
2. a lens bridging the previous version to it, and
3. a frozen fixture for the new version.

`Passport::from_stored` then upcasts a stored document through the registered
lens chain before deserialising it.

**A lens may honestly refuse.** Battery documents before `v2.5.0` are a
documented refusal rather than an oversight: `v2.5.0` made `batteryType` required
because Annex VI Part A point 2 makes the battery category mandatory public
content, and no lens can invent a category for a record that predates the
mandate. Refusing is the correct outcome; inventing a value would be a fabricated
regulatory claim.

**None of this reaches the envelope.** There is no envelope lens, so an envelope
change has exactly one sanctioned form: additive, per §3.

---

## 5. The second surface — stored is not the only one

A field in the **signed public view** is on disk *and* on the wire between
operators, and the two surfaces want opposite failure behaviour.

| | Stored document | Fetched document |
|---|---|---|
| Whose bytes | this node's own database | another operator's node |
| Signed? | yes | yes |
| Can it be rewritten? | **yes** — by its owner | **never, by anyone** |
| Right failure mode | **loud** — names a problem whose owner can fix it | **tolerant** — it must keep parsing indefinitely |

For a stored document, refusing to read is a good failure: it is visible, and the
node that owns the bytes can migrate them.

For a fetched one it is not. A passport fetched from another operator is signed
and belongs to them. It cannot be migrated into a new shape — not by us, not by
them without breaking their own signature. **A reader that refuses it refuses
data that is correct, current and unforgeable, permanently.**

And refusal does not stay quiet. The verification walk that reads a fetched
passport's component references reports anything it cannot parse as a malformed
reference, and the evidence dossier grades a malformed reference as an integrity
violation, alongside a hash mismatch and a cycle. So a node that upgraded would
report a node that had not as having a **tampered** bill of materials, on
evidence that is nothing but a version difference. Nodes here are independent
per-operator deployments, so that skew is the normal steady state, not a
migration window.

**Rule:** for any field in the signed public view, decide compatibility from the
*fetched* column. It is strictly stricter than the stored one, and the stored
analysis alone has already produced the wrong answer.

---

## 6. What is enforced, and what is not

`schema_compat.rs` holds one frozen document per `(product group, schema
version)` and asserts each still reads through `from_stored`. A fixture is
written once and never regenerated, because — in the file's own words — *a
fixture regenerated from the current schema would agree with the current schema
by construction and could not catch anything.*

That is a real tripwire, and it covers `productGroupData`.

🚨 **It gives the envelope no coverage at all.** The fixtures are
`productGroupData`-shaped; the surrounding envelope is built by `stored_passport`
as a **JSON literal written in whatever the current shape is**. So the envelope
half of every fixture is exactly the thing the file warns against — regenerated
to agree with the present — and a renamed envelope key passes this test without
noticing.

That is why both envelope renames in §1 shipped. It is not an argument for
weakening the rule; it is the reason the envelope rule has to be followed by
hand, and the reason a reviewer should treat any diff touching `Passport`'s
fields or `PASSPORT_WIRE_KEYS` as needing an explicit compatibility argument
rather than a green test run.

Closing this gap means a frozen *envelope* fixture, independent of product group.
Worth doing; not done.

---

## 7. If you have to break it anyway

Sometimes the right change is non-additive and there is no lens — an envelope
rename, most obviously. The rule then is not "never", it is **"say so, and say
what it costs"**:

- State it in `CHANGELOG.md` under `### Breaking`, with the migration a holder of
  existing documents must perform.
- Say which failure mode it has: refusing to load, or loading with the field
  silently missing. The second is worse and must never be left unstated.
- Record why it was acceptable *now*. Every such change so far has rested on
  there being no published passports to strand. **That licence expires at the
  first real one**, and after that an envelope rename must carry the old key or a
  one-time document rewrite in the publish pipeline.
- Remember that a rewritten document no longer verifies against a signature that
  covers the old key names. There is no version of an envelope rename that a
  signed, published passport survives without a migration written for it.

The honest summary is that this rule is cheap to follow now and becomes
impossible to retrofit later, which is the whole reason it is written down before
anyone has been hurt by it in production.
