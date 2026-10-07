//! Emit every JSON-LD context and document this workspace builds, for an
//! independent JSON-LD 1.1 processor to expand.
//!
//! # Why this exists
//!
//! `jsonld_context.rs` holds the context to this repository's own reading of the
//! JSON-LD specification, and it says so: the workspace has no processor, so it
//! asserts terms instead of running an expansion. A transcription error in a
//! term, or a key emitted at a position no term reaches, passes it. In JSON-LD
//! the second kind is the defect tooling hides best: a key with no term is
//! dropped on expansion together with everything inside it, with no error.
//!
//! An independent processor is the only check that is not circular. This test
//! produces the corpus; `.github/scripts/jsonld_oracle.py` expands it with PyLD
//! and fails on any processor error and on any property that expands to
//! nothing.
//!
//! # What is in the corpus
//!
//! - every **context** an emitted document carries, as the `@context` value:
//!   the passport context, the passport credential's, the access credential's,
//!   and the DID document's;
//! - a **document** for each thing that carries one, always built by the real
//!   builder or serialiser: a DID document at a first key and after a rotation,
//!   a passport credential, an access credential with and without a status
//!   entry, and a framed passport for each product group in the catalog.
//!
//! A framed passport's `productGroupData` is the group's schema-compat fixture
//! for its current schema version. Those fixtures carry the required keys of
//! each product group, so the keys the corpus exercises are the ones every
//! passport of that group has.
//!
//! # Running
//!
//! ```text
//! EMIT_JSONLD_CORPUS=1 cargo test -p dpp-tests --test jsonld_oracle_corpus
//! ```
//!
//! Writes `target/jsonld-oracle/corpus.json`. Without the variable the test
//! still runs and still checks that the corpus is complete, so an ordinary
//! `just check` gets that coverage without producing files.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use dpp_crypto::keystore::KeyStore;
use dpp_domain::{
    PassportCredential, PassportCredentialSubject, ProductGroup, ProductGroupCatalog,
    ProductGroupData,
};
use dpp_tests::fixtures::{base_passport, make_subject};
use dpp_vc::credential::{CredentialBuilder, CredentialRole, CredentialStatus};
use dpp_vc::did_builder::build_did_document;
use dpp_vc::{context_value, frame_passport};
use serde_json::{Value, json};

const ISSUER: &str = "did:web:issuer.example.com";

/// A passport's wire form with its product group data replaced by `data`.
///
/// The envelope comes from the real serialiser, so every envelope key the
/// passport emits is present. Only `productGroupData` is swapped, and it is
/// swapped as JSON rather than deserialised into a typed variant, so the corpus
/// holds exactly the keys the fixture holds.
fn framed_passport(group: &ProductGroup, data: Value) -> Value {
    let placeholder: ProductGroupData = serde_json::from_value(json!({
        "productGroup": "packaging",
        "materialFamily": "placeholder",
    }))
    .expect("an unmodelled product group deserialises");

    let mut passport = serde_json::to_value(base_passport(group.clone(), placeholder, "1.0.0"))
        .expect("a passport serialises");

    let tag = serde_json::to_value(group).expect("a product group serialises");
    let mut data = data;
    data.as_object_mut()
        .expect("product group data is an object")
        .insert("productGroup".to_owned(), tag.clone());

    let wire = passport.as_object_mut().expect("a passport is an object");
    wire.insert("productGroup".to_owned(), tag);
    wire.insert("productGroupData".to_owned(), data);

    frame_passport(passport)
}

/// The group's fixture at its current schema version.
fn fixture_for(key: &str, version: &str) -> Value {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../dpp-domain/tests/fixtures/schema-compat")
        .join(key)
        .join(format!("v{version}.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("no schema-compat fixture for '{key}' at {version}: {e}"));
    serde_json::from_str(&raw).expect("a fixture is JSON")
}

/// A DID document built the way a deployment builds one, at a first key and
/// after a hygiene rotation.
fn did_documents() -> Vec<(String, Value)> {
    let mut documents = Vec::new();
    for (note, rotations, base_url) in [
        (
            "a DID document at a first key",
            0,
            "https://first.example.com",
        ),
        (
            "a DID document after a rotation",
            1,
            "https://rotated.example.com",
        ),
    ] {
        // Cargo's scratch directory for integration tests, inside `target`, and not
        // the system temporary directory, which other users share.
        let path = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("test-jsonld-corpus-{}.json", uuid::Uuid::now_v7()));
        let store = KeyStore::open(&path, "test-pass").expect("open the keystore");
        store
            .generate_key("issuer")
            .expect("generate the first key");
        for _ in 0..rotations {
            store.rotate_key("issuer").expect("rotate");
        }
        let document = build_did_document(&store, base_url, "issuer").expect("build the document");
        let _ = std::fs::remove_file(&path);
        documents.push((note.to_owned(), document));
    }
    documents
}

/// A thing the corpus holds, and the words that name it in a failure.
type Labelled = Vec<(String, Value)>;

/// The corpus: every context, then every document.
fn corpus() -> (Labelled, Labelled) {
    let mut contexts: Labelled = Vec::new();
    let mut documents: Labelled = Vec::new();

    contexts.push(("the passport context".into(), context_value()));

    for (note, document) in did_documents() {
        contexts.push((
            format!("the @context of {note}"),
            document["@context"].clone(),
        ));
        documents.push((note, document));
    }

    let passport_credential = PassportCredential::new(
        ISSUER.into(),
        PassportCredentialSubject {
            id: "urn:uuid:00000000-0000-7000-8000-000000000000".into(),
            payload_hash: "deadbeef".into(),
        },
    );
    contexts.push((
        "the passport credential's @context".into(),
        Value::Array(passport_credential.context.clone()),
    ));
    documents.push((
        "a passport credential".into(),
        serde_json::to_value(&passport_credential).expect("serialises"),
    ));

    let subject = make_subject(
        "did:web:holder.example.com",
        "Test Holder",
        CredentialRole::AuthorisedRepairer,
        vec!["textile".into()],
    );
    let access = CredentialBuilder::new(ISSUER.into(), subject.clone()).build();
    contexts.push((
        "the access credential's @context".into(),
        Value::Array(access.context.clone()),
    ));
    documents.push((
        "an access credential".into(),
        serde_json::to_value(&access).expect("serialises"),
    ));

    let with_status = CredentialBuilder::new(ISSUER.into(), subject)
        .with_status(CredentialStatus {
            id: "https://issuer.example.com/status/1#7".into(),
            status_type: "BitstringStatusListEntry".into(),
            status_purpose: "revocation".into(),
            status_list_index: Some("7".into()),
            status_list_credential: Some("https://issuer.example.com/status/1".into()),
        })
        .build();
    documents.push((
        "an access credential with a status entry".into(),
        serde_json::to_value(&with_status).expect("serialises"),
    ));

    for descriptor in ProductGroupCatalog::new().all().iter() {
        let group = ProductGroup::from_wire_tag(&descriptor.key);
        let data = fixture_for(&descriptor.key, &descriptor.current_schema_version);
        documents.push((
            format!("a framed {} passport", descriptor.key),
            framed_passport(&group, data),
        ));
    }

    (contexts, documents)
}

#[test]
fn the_corpus_covers_every_context_and_product_group() {
    let (contexts, documents) = corpus();

    // The remote contexts the oracle must be able to serve are the ones the
    // corpus actually references, so a context added to a builder reaches the
    // oracle's own check without anybody listing it here.
    let mut remote: BTreeSet<&str> = BTreeSet::new();
    for (_, context) in &contexts {
        for entry in context.as_array().expect("every context is an array") {
            if let Some(url) = entry.as_str() {
                remote.insert(url);
            }
        }
    }
    assert!(
        remote.contains("https://www.w3.org/ns/credentials/v2")
            && remote.contains("https://www.w3.org/ns/did/v1"),
        "the corpus no longer references the W3C contexts it was written to exercise: {remote:?}"
    );

    let catalog = ProductGroupCatalog::new();
    for descriptor in catalog.all().iter() {
        let label = format!("a framed {} passport", descriptor.key);
        assert!(
            documents.iter().any(|(note, _)| *note == label),
            "the corpus holds no passport for product group '{}'",
            descriptor.key
        );
    }

    for (note, document) in &documents {
        assert!(
            document.get("@context").is_some(),
            "'{note}' carries no @context, so it is not JSON-LD"
        );
    }

    if std::env::var_os("EMIT_JSONLD_CORPUS").is_none() {
        return;
    }

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/jsonld-oracle");
    std::fs::create_dir_all(&dir).expect("create the oracle output directory");
    let body = json!({
        "contexts": contexts
            .iter()
            .map(|(label, context)| json!({ "label": label, "context": context }))
            .collect::<Vec<_>>(),
        "documents": documents
            .iter()
            .map(|(label, document)| json!({ "label": label, "document": document }))
            .collect::<Vec<_>>(),
    });
    std::fs::write(
        dir.join("corpus.json"),
        serde_json::to_string_pretty(&body).expect("serialises"),
    )
    .expect("write the corpus");
    eprintln!(
        "wrote {} contexts and {} documents to {}",
        contexts.len(),
        documents.len(),
        dir.display()
    );
}
