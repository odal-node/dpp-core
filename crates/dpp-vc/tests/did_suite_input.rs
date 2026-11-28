//! Produce DID documents with `build_did_document`, in the input format of the
//! W3C DID test suite.
//!
//! # Why this exists
//!
//! Every test of the DID document builder was written by the people who wrote the
//! builder, so a misreading of DID Core that the builder and its tests share
//! passes both. The W3C publishes a test suite, `w3c/did-test-suite`, that checks
//! the data model rules of §5, the production rules of §6 and the metadata
//! structure of §7.3 against documents an implementation hands it. That is the
//! only check available here that is not circular. This test produces the
//! documents; `.github/scripts/did_suite_oracle.mjs` runs the suite over them.
//!
//! # What is handed over, and what is not
//!
//! The documents are exactly what `build_did_document` returns, never written by
//! hand, for the states a deployment reaches: a first key, a hygiene rotation, a
//! compromise, both of those in one history, a host with a port, and a current key
//! that is revoked, beside an archived key and alone. A store's own calls never leave
//! its current key revoked; one written before key stores carried an integrity tag
//! can, and `KeyStore::open_and_migrate` opens it, so that is the store used. Each is
//! given to the suite as the JSON-LD representation, `application/did+ld+json`,
//! because that is what they are: every one carries `@context`, which DID Core
//! 6.3 defines as the representation-specific entry of that representation. The
//! plain JSON representation is not produced by this library, so it is not
//! offered.
//!
//! `@context` is split out of the data model on that basis, as the suite's own
//! example implementation does. The rest of the document is the data model.
//!
//! # Scope
//!
//! The suite checks the *document*. It does not check that a `did:web` DID
//! resolves, since that method's specification is not a W3C standard, and it does
//! not check the verification method type, which is judged separately.
//!
//! # Running
//!
//! ```text
//! EMIT_DID_SUITE_INPUT=1 cargo test -p dpp-vc --test did_suite_input
//! ```
//!
//! Writes `target/did-oracle/implementation.json`. Without the variable the test
//! still runs and still checks our own side, so an ordinary `just check` gets the
//! coverage without producing files.

use std::collections::BTreeSet;

use serde_json::{Map, Value, json};

use dpp_crypto::keystore::KeyStore;
use dpp_vc::did_builder::build_did_document;

#[path = "key_store_support/legacy_store.rs"]
mod legacy_store;

const KEY_ID: &str = "issuer";

/// What the keystore holds before a scenario's history is played into it.
#[derive(Clone, Copy)]
enum Start {
    /// Nothing: the caller provisions a first key before the history.
    Empty,
    /// A current key revoked in a store written before key stores carried an
    /// integrity tag, opened and sealed by `KeyStore::open_and_migrate`, with an
    /// archived key that is not revoked or without one.
    RevokedCurrent { with_archived_key: bool },
}

/// What happens to an issuer's keys before its document is built.
#[derive(Clone, Copy)]
enum Step {
    /// Hygiene rotation: the old key stays listed, so older signatures verify.
    Rotate,
    /// Compromise: the old key is revoked and drops out of the document.
    RevokeAndRotate,
}

struct Scenario {
    note: &'static str,
    base_url: &'static str,
    start: Start,
    history: &'static [Step],
    /// The verification methods the document must list, from what the keystore
    /// holds: one per current or archived key that has not been revoked.
    methods: usize,
}

/// Distinct hosts, so each document has its own DID: the suite keys a document
/// by its DID, and two states of one issuer would otherwise overwrite each other.
const SCENARIOS: &[Scenario] = &[
    Scenario {
        note: "a first key, explicitly provisioned before building the document",
        base_url: "https://first.example.com",
        start: Start::Empty,
        history: &[],
        methods: 1,
    },
    Scenario {
        note: "a hygiene rotation keeps the old key listed",
        base_url: "https://rotated.example.com",
        start: Start::Empty,
        history: &[Step::Rotate],
        methods: 2,
    },
    Scenario {
        note: "a compromise drops the revoked key",
        base_url: "https://revoked.example.com",
        start: Start::Empty,
        history: &[Step::RevokeAndRotate],
        methods: 1,
    },
    Scenario {
        note: "a retained key and a revoked one in the same history",
        base_url: "https://mixed.example.com",
        start: Start::Empty,
        history: &[Step::Rotate, Step::RevokeAndRotate],
        methods: 2,
    },
    Scenario {
        note: "a host with a port, whose colon is percent-encoded in the DID",
        base_url: "https://port.example.com:8443",
        start: Start::Empty,
        history: &[],
        methods: 1,
    },
    Scenario {
        note: "a revoked current key drops out and leaves no authentication",
        base_url: "https://revoked-current.example.com",
        start: Start::RevokedCurrent {
            with_archived_key: true,
        },
        history: &[],
        methods: 1,
    },
    Scenario {
        note: "a store whose only key is revoked publishes no key",
        base_url: "https://no-key.example.com",
        start: Start::RevokedCurrent {
            with_archived_key: false,
        },
        history: &[],
        methods: 0,
    },
];

/// A document as `build_did_document` returns it, after the key history of
/// `scenario` has been played into a fresh keystore.
fn produce(scenario: &Scenario) -> Value {
    let (store, path) = match scenario.start {
        Start::Empty => {
            let path = std::env::temp_dir().join(format!(
                "test-did-suite-input-{}.json",
                uuid::Uuid::now_v7()
            ));
            let store = KeyStore::open(&path, "test-pass").expect("open the keystore");
            (store, path)
        }
        Start::RevokedCurrent { with_archived_key } => {
            let path =
                legacy_store::with_revoked_current("did-suite-input", KEY_ID, with_archived_key);
            let store = KeyStore::open_and_migrate(&path, legacy_store::PASSPHRASE)
                .expect("open and seal the keystore");
            (store, path)
        }
    };

    // The builder only reads keys, so a fresh store needs explicit provisioning.
    if matches!(scenario.start, Start::Empty) {
        store.generate_key(KEY_ID).expect("generate the first key");
    }
    for step in scenario.history {
        match step {
            Step::Rotate => store.rotate_key(KEY_ID).expect("rotate"),
            Step::RevokeAndRotate => store.revoke_and_rotate(KEY_ID).expect("revoke and rotate"),
        };
    }

    let doc = build_did_document(&store, scenario.base_url, KEY_ID).expect("build the document");
    let _ = std::fs::remove_file(&path);
    doc
}

/// One document in the suite's per-DID shape: the data model, then the
/// representation, which is the document serialised exactly as produced.
fn suite_entry(doc: &Value) -> Value {
    let mut properties = doc
        .as_object()
        .expect("a DID document is an object")
        .clone();
    let context = properties
        .remove("@context")
        .expect("the document carries @context");

    json!({
        "didDocumentDataModel": { "properties": properties },
        "application/did+ld+json": {
            "didDocumentDataModel": {
                "representationSpecificEntries": { "@context": context }
            },
            "representation": doc.to_string(),
            "didDocumentMetadata": {},
            "didResolutionMetadata": { "contentType": "application/did+ld+json" }
        }
    })
}

/// The whole input file: one implementation with every produced DID.
fn implementation() -> Value {
    let mut file = Map::new();
    let mut dids = Vec::new();
    let mut entries = Map::new();

    for scenario in SCENARIOS {
        let doc = produce(scenario);
        let did = doc["id"].as_str().expect("id is a string").to_owned();
        entries.insert(did.clone(), suite_entry(&doc));
        dids.push(did);
    }

    file.insert("didMethod".into(), json!("did:web"));
    file.insert(
        "implementation".into(),
        json!("dpp-vc did:web document builder"),
    );
    file.insert("implementer".into(), json!("Odal Node"));
    file.insert(
        "supportedContentTypes".into(),
        json!(["application/did+ld+json"]),
    );
    file.insert("dids".into(), json!(dids));
    file.insert("didParameters".into(), json!({}));
    file.extend(entries);
    Value::Object(file)
}

/// Our own side must hold before the documents are worth handing to anyone.
///
/// A scenario that stopped producing the state it names would make the suite
/// pass over a narrower set of documents than this file claims.
#[test]
fn each_scenario_produces_the_state_it_names() {
    let mut dids = BTreeSet::new();

    for scenario in SCENARIOS {
        let doc = produce(scenario);

        // A property with nothing to list is left out, never empty: DID Core 1.0
        // §5.3.1 and §5.3.2 require "one or more" verification methods when
        // `authentication` or `assertionMethod` is present.
        assert_eq!(
            doc.get("verificationMethod")
                .map(|vm| vm.as_array().map(Vec::len)),
            (scenario.methods > 0).then_some(Some(scenario.methods)),
            "{}: wrong number of verification methods",
            scenario.note
        );
        assert_eq!(
            doc.get("assertionMethod").is_some(),
            scenario.methods > 0,
            "{}: assertionMethod is present exactly when a key is",
            scenario.note
        );
        assert_eq!(
            doc.get("authentication").is_some(),
            matches!(scenario.start, Start::Empty),
            "{}: authentication names the current key, and only an unrevoked one",
            scenario.note
        );
        assert_eq!(
            doc["@context"][0], "https://www.w3.org/ns/did/v1",
            "{}: DID Core 6.3.1 puts its own context first",
            scenario.note
        );
        assert!(
            dids.insert(doc["id"].as_str().expect("id is a string").to_owned()),
            "{}: two scenarios share a DID, so one would overwrite the other",
            scenario.note
        );
    }

    // The port's colon must have been percent-encoded, or the scenario that
    // exists to cover that is not covering it.
    assert!(
        dids.contains("did:web:port.example.com%3A8443"),
        "the port scenario did not produce a percent-encoded colon: {dids:?}"
    );
}

/// Write the input file for the W3C suite, when asked.
#[test]
fn emit_input_for_the_w3c_did_suite() {
    let file = implementation();
    assert_eq!(
        file["dids"].as_array().map(Vec::len),
        Some(SCENARIOS.len()),
        "every scenario must reach the suite"
    );

    if std::env::var_os("EMIT_DID_SUITE_INPUT").is_none() {
        return;
    }

    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/did-oracle");
    std::fs::create_dir_all(&dir).expect("create the oracle output directory");
    let path = dir.join("implementation.json");
    std::fs::write(&path, file.to_string()).expect("write the suite input");
    eprintln!(
        "wrote {} DID documents to {}",
        SCENARIOS.len(),
        path.display()
    );
}
