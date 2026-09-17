//! Fixtures shared by this module's test files.
//!
//! Split out when `tests.rs` outgrew the navigable size and divided along the
//! seams its source already had — issuance, verification, issuer metadata. The
//! fixtures are common to all three, so they live beside them rather than being
//! duplicated into each.

use base64::Engine;
use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Value, json};

use dpp_crypto::keystore::KeyStore;
use dpp_domain::ProductGroup;
use dpp_domain::access::ProductGroupAccessPolicy;

use super::{SdJwtVcError, issue, vct_for, verify};

pub(super) const KEY_ID: &str = "sd-jwt-issuer";
pub(super) const ISSUER: &str = "https://passports.operator.example";
pub(super) const SCHEMA_VERSION: &str = "2.6.0";
/// The fixture's `iat`. A fixed instant, so nothing here depends on the wall
/// clock and the validity-window tests can name a moment either side of it.
pub(super) const ISSUED_AT: i64 = 1_789_000_000;

pub(super) fn now() -> DateTime<Utc> {
    Utc.timestamp_opt(ISSUED_AT + 60, 0).unwrap()
}

pub(super) fn policy() -> ProductGroupAccessPolicy {
    ProductGroupAccessPolicy::for_passport("battery", SCHEMA_VERSION)
        .expect("battery 2.6.0 schema is embedded")
}

/// A passport-shaped payload: an envelope with a product-group payload beneath
/// it, carrying fields from each of the four disclosure classes.
pub(super) fn payload() -> Value {
    json!({
        "id": "018f3a4c-0000-7000-8000-000000000001",
        "productGroup": "battery",
        "schemaVersion": SCHEMA_VERSION,
        "batchId": "BATCH-7781",
        "productGroupData": {
            "batteryChemistry": "LFP",
            "cathodeMaterial": ["LiFePO4"],
            "stateOfHealthPct": 87.5,
            "testReportResults": "report-4471",
            "safetyMeasures": ["Do not puncture"],
        }
    })
}

pub(super) fn public_key_b64(store: &KeyStore) -> String {
    let info = store.public_key(KEY_ID).expect("key present");
    // `verify_jws` decodes with URL_SAFE_NO_PAD, so the caller must encode the
    // same way — a STANDARD-encoded key silently fails to verify.
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(hex::decode(&info.verifying_key_hex).unwrap())
}

/// Resolve claim names to the digests `present` selects by.
pub(super) fn digests(credential: &dpp_crypto::sd_jwt::SdJwt, names: &[&str]) -> Vec<String> {
    names
        .iter()
        .flat_map(|n| credential.digests_for_claim(n))
        .collect()
}

pub(super) fn issued(store: &KeyStore) -> dpp_crypto::sd_jwt::SdJwt {
    issue(
        store,
        KEY_ID,
        &payload(),
        &policy(),
        ISSUER,
        &vct_for(ProductGroup::Battery, SCHEMA_VERSION),
        ISSUED_AT,
    )
    .expect("issue")
}

/// Issue with extra top-level claims merged into the fixture payload.
///
/// Lets a validity window be put on a credential that is otherwise exactly the
/// one every other test here uses.
pub(super) fn issued_with(store: &KeyStore, extra: &[(&str, Value)]) -> String {
    let mut payload = payload();
    let object = payload.as_object_mut().unwrap();
    for (k, v) in extra {
        object.insert((*k).to_owned(), v.clone());
    }
    issue(
        store,
        KEY_ID,
        &payload,
        &policy(),
        ISSUER,
        &vct_for(ProductGroup::Battery, SCHEMA_VERSION),
        ISSUED_AT,
    )
    .expect("issue")
    .serialise()
}

/// Verify at a given instant, discarding the payload.
pub(super) fn check(
    store: &KeyStore,
    credential: &str,
    at: DateTime<Utc>,
) -> Result<(), SdJwtVcError> {
    verify(
        credential,
        &public_key_b64(store),
        &vct_for(ProductGroup::Battery, SCHEMA_VERSION),
        at,
    )
    .map(|_| ())
}
