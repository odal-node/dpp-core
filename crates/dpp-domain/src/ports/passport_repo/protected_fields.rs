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
    // The applicable law at placing on the market does not change. A
    // mis-recorded set is corrected by superseding the passport, never by
    // patching a published record's legal basis.
    "applicableInstruments",
];
