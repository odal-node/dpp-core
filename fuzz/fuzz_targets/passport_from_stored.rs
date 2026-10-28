#![no_main]
//! Fuzz reading a stored passport document — `Passport::from_stored` reads every
//! document a host holds, including ones written by older versions and ones
//! fetched from another operator, through the direct shape and the lens chain.
//! Property: it returns `Ok`/`Err` for any JSON document, never panics.

use std::sync::OnceLock;

use dpp_domain::catalog::ProductGroupCatalog;
use dpp_domain::schemas::LensRegistry;
use dpp_domain::Passport;
use libfuzzer_sys::fuzz_target;

fn registries() -> &'static (LensRegistry, ProductGroupCatalog) {
    static REGISTRIES: OnceLock<(LensRegistry, ProductGroupCatalog)> = OnceLock::new();
    REGISTRIES.get_or_init(|| (LensRegistry::new(), ProductGroupCatalog::new()))
}

fuzz_target!(|data: &[u8]| {
    if let Ok(doc) = serde_json::from_slice::<serde_json::Value>(data) {
        let (lenses, catalog) = registries();
        let _ = Passport::from_stored(doc, lenses, catalog);
    }
});
