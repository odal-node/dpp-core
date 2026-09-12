//! [`PROTECTED_PATCH_FIELDS`] — the keys a field patch may never reach.

/// Fields governed by the state machine, the retention lock, the publish/seal
/// pipeline, record identity, or a dedicated transition method — none of
/// which is a user-editable content field. `patch_fields` rejects any delta
/// touching one of these so it cannot be used to bypass `transition_to`/
/// `update_status` (e.g. flip `retentionLocked` back to `false` or forge a
/// `jwsSignature`), or `RegistrySyncPort::notify_transfer` (change
/// `operatorIdentifier` without going through transfer-of-responsibility).
/// `facility` is likewise excluded: it is a point-in-time snapshot copied at
/// create time by design, not a field any flow updates in place. Serialized
/// (camelCase) field names, matching the `Passport` JSON representation.
///
/// # Why this is `pub`
///
/// It is part of this trait's documented behaviour — the `patch_fields` doc
/// below tells a caller which keys are refused — and a backend that overrides
/// the default implementation still owes callers that contract. Keeping it
/// private meant an implementor had no way to honour it except by retyping the
/// list, and a retyped list drifts: the PostgreSQL backend's copy fell three
/// entries short (`operatorIdentifier`, `facility`, and `parentPassportRef` as
/// the upward lineage edge was then called — now `derivedFrom`), which
/// on the only backend that ships made those fields writable through a
/// user-facing field patch and carried them into the signed publish payload.
///
/// An implementation that must differ should derive its list *from this value*
/// — adding or removing named entries with the reason stated — never restate it.
///
/// A slice rather than a fixed-size array on purpose: the length is not part of
/// the type, so adding an entry here does not break a consumer that annotated
/// one.
pub const PROTECTED_PATCH_FIELDS: &[&str] = &[
    "id",
    "status",
    "retentionLocked",
    "retentionUntil",
    "jwsSignature",
    "publicJwsSignature",
    // The fourth proof field, alongside the two signatures and the seal. It was
    // the only one of the four missing here while the PostgreSQL backend's copy
    // protected it — so this list, the one every other implementation inherits,
    // was the weaker of the two.
    "disclosureSignatures",
    "seal",
    "version",
    "publishedAt",
    "createdAt",
    "supersedesId",
    "schemaVersion",
    "operatorIdentifier",
    "facility",
    "derivedFrom",
    "componentRefs",
    // Four of its five values are set at create, because each Art. 77(7)
    // operation produces a new passport. `waste` is the one transition that
    // happens to a record that continues — and it is also a responsibility move
    // under Art. 77(7)'s second subparagraph, so it is not a free patch field
    // either. Protected here so the transition goes through `supersedes_id` +
    // `version`: an explicit versioning event with an audit trail, rather than a
    // rewrite of a body that has already been signed.
    "lifeStatus",
    // The applicable law at placing on the market does not change. A
    // mis-recorded set is corrected by superseding the passport, never by
    // patching a published record's legal basis.
    "applicableInstruments",
    // ── Added after an audit found the deny-list default had let ten modelled
    // envelope fields through. None was ever reachable via `PUT /dpp/{id}` on
    // the only consumer that ships — it builds its delta from an allow-list —
    // but that guard lives in a consumer, and this list is the contract every
    // other implementor inherits. See `protected_fields_tests`, which now makes
    // an unclassified wire key a compile-time-adjacent failure rather than a
    // silent permission.
    //
    // The registry independently checks this one. IR (EU) 2026/1778 Art. 8(7)(c)
    // has the Commission confirm a passport's conformity with the granularity
    // level it was registered at, and Arts. 8(4)-(5) hang the batch and model
    // identifier links off that level. A level that can move after registration
    // desynchronises the record from a check already performed on it.
    "granularity",
    // Decides which schema validates the passport and which instruments apply.
    // Changing it on a published record reinterprets the whole document.
    "productGroup",
    // Product identity. Same class as `id`, one level down.
    "productId",
    // Registration data the registry validates against the commodity-code ranges
    // its product group permits — IR (EU) 2026/1778 Art. 8(7)(d).
    "commodityCode",
    // Every effectivity and retention calculation keys on this date. Moving it
    // silently moves which obligations a passport is judged against.
    "placedOnMarketDate",
    // The Annex III(k) actor, and the odd one out until now: `operatorIdentifier`
    // and `facility`, its two neighbours in the same snapshot, were already here.
    "responsibleOperator",
    // Which physical units the passport covers. Set at create.
    "batchId",
    // Who made the product — a point-in-time fact, like `facility`, not a field
    // any flow updates in place.
    "manufacturer",
    // The carrier's address. Repointing it silently redirects a QR code already
    // printed on a physical product.
    "qrCodeUrl",
    // Record metadata owned by the repository, alongside `createdAt`,
    // `publishedAt` and `version` — all three of which were already protected.
    // Leaving this one writable let a caller forge the modification time.
    "updatedAt",
    // The material composition, and the same argument as `componentRefs`: it is
    // inside the signed public view, so changing a published one is a new
    // passport version via `supersedesId`, not an in-place edit. Patching it
    // would leave the served body no longer verifying against its own signature.
    "materials",
];
