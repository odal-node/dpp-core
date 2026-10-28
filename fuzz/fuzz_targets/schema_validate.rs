#![no_main]
//! Fuzz schema validation — a create body is untrusted JSON, and it is checked
//! against a product group's schema before anything else reads it, patterns and
//! formats included.
//! Property: validating any JSON value against the newest schema of every
//! product group returns `Ok`/`Err`, never panics.

use std::sync::OnceLock;

use dpp_domain::schemas::VersionedSchemaRegistry;
use libfuzzer_sys::fuzz_target;

fn registry() -> &'static VersionedSchemaRegistry {
    static REGISTRY: OnceLock<VersionedSchemaRegistry> = OnceLock::new();
    REGISTRY.get_or_init(VersionedSchemaRegistry::new)
}

fuzz_target!(|data: &[u8]| {
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(data) {
        let schemas = registry();
        for group in schemas.product_groups() {
            if let Some((version, _)) = schemas.latest(group) {
                let _ = schemas.validate(group, version, &value);
            }
        }
    }
});
