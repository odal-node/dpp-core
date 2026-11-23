//! Status List Tokens for an independent Token Status List library to read, and the
//! tokens that library issues, for `dpp_vc::token_status_list` to verify.
//!
//! # Why this exists
//!
//! The unit tests hold the module to this repository's own reading of
//! draft-ietf-oauth-status-list-21 and to the draft's own examples. A misreading the
//! author made twice passes both. `@owf/token-status-list` shares no code and no
//! author with this crate, so a list it writes that this crate reads differently, or
//! the other way round, is a clause to read.
//!
//! Judged at the level of the **list and the token**: the bit order, the widths, the
//! zlib stream, the claims, `sub` against `uri`, `exp`. The draft is not an RFC, and
//! this workspace claims no conformance to it, so none of this is evidence of any.
//!
//! # What the other implementation checks
//!
//! `.github/oracle/tokenstatuslist/judge.mjs` reads the corpus. A token this crate
//! accepts must come out of the library's reading with the entries given here, and a
//! token this crate refuses must be refused by the library or by Node's `crypto`,
//! which checks every signature. The draft's Appendix C vectors are read by both.
//!
//! # The other direction
//!
//! `CHECK_STATUS_LIST_ORACLE_ISSUED` names a file of tokens the library issued, with
//! the key each is signed under and the value of every entry it asked about.
//! `StatusListToken::verify` has to accept each, and `status` has to read each entry
//! as the library wrote it.
//!
//! # Running
//!
//! ```text
//! EMIT_STATUS_LIST_CORPUS=1 cargo test -p dpp-tests --test status_list_oracle_corpus
//! CHECK_STATUS_LIST_ORACLE_ISSUED=target/status-list-oracle/issued.json \
//!   cargo test -p dpp-tests --test status_list_oracle_corpus
//! ```
//!
//! Without the variables the tests still check that every case reads as its `accepts`
//! says, so an ordinary `just check` gets that coverage without producing files.

use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use chrono::{DateTime, TimeZone, Utc};
use dpp_crypto::jws::{KeyAlgorithm, PublicKey};
use dpp_crypto::keystore::KeyStore;
use dpp_vc::token_status_list::{
    Status, StatusListToken, StatusReference, TYP, TokenStatusList, TokenStatusListError,
};
use serde_json::{Value, json};

const URI: &str = "https://status.example/statuslists/1";
/// A fixed instant, so nothing here depends on the wall clock.
const ISSUED_AT: i64 = 1_789_000_000;
const NOW: i64 = ISSUED_AT + 60;
/// The entries of every list below: a whole number of bytes at every width.
const ENTRIES: u64 = 64;
/// A gzip stream, which is the W3C list's compression and not this draft's.
const GZIP_OF_ONE_ZERO_BYTE: [u8; 25] = [
    0x1f, 0x8b, 0x08, 0, 0, 0, 0, 0, 0, 0x03, 0x63, 0, 0, 0x8d, 0xef, 0x02, 0xd2, 0x01, 0, 0, 0, 0,
    0, 0, 0,
];

const KEYS: [(&str, KeyAlgorithm); 4] = [
    ("eddsa", KeyAlgorithm::Ed25519),
    ("es256", KeyAlgorithm::EcdsaP256),
    ("es384", KeyAlgorithm::EcdsaP384),
    ("es512", KeyAlgorithm::EcdsaP521),
];
/// A key that signs a token claimed to be another's.
const OTHER_KEY: &str = "other";

fn now() -> DateTime<Utc> {
    Utc.timestamp_opt(NOW, 0).unwrap()
}

struct Signers {
    store: KeyStore,
    path: PathBuf,
}

impl Signers {
    fn new() -> Self {
        let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "test-status-list-corpus-{}.json",
            uuid::Uuid::now_v7()
        ));
        // A new passphrase per run, for a store that lives for one test.
        let store = KeyStore::open(&path, &uuid::Uuid::now_v7().to_string()).expect("open");
        for (id, algorithm) in KEYS {
            store.generate_key_with(id, algorithm).expect("a key");
        }
        store.generate_key(OTHER_KEY).expect("a key");
        Self { store, path }
    }

    fn public_key(&self, id: &str) -> PublicKey {
        self.store.load_key(id).expect("a key").public_key()
    }

    /// A Status List Token of `claims`, signed by `id`, typed `typ`.
    fn token(&self, id: &str, typ: Option<&str>, claims: &Value) -> String {
        let kid = self.public_key(id).thumbprint_uri();
        dpp_crypto::jws::sign_typed(&self.store, id, claims, &kid, typ, None).expect("signs")
    }
}

impl Drop for Signers {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// The claims of a Status List Token around `list`, as the draft's example has them.
fn claims(list: &TokenStatusList) -> Value {
    json!({
        "sub": URI,
        "iat": ISSUED_AT,
        "exp": ISSUED_AT + 86_400,
        "ttl": 43_200,
        "status_list": { "bits": list.bits(), "lst": list.encode() },
    })
}

/// `claims` with one member replaced, or removed when `value` is `None`.
fn with(mut claims: Value, name: &str, value: Option<Value>) -> Value {
    let object = claims.as_object_mut().expect("claims are an object");
    match value {
        Some(value) => object.insert(name.to_owned(), value),
        None => object.remove(name),
    };
    claims
}

/// A list of 16 two-bit entries holding the three statuses and a gap.
fn small_list() -> TokenStatusList {
    let mut list = TokenStatusList::new(2, 16).unwrap();
    for (idx, value) in [(1, 1), (2, 2), (3, 3), (9, 1)] {
        list.set(idx, value).unwrap();
    }
    list
}

fn case(
    name: &str,
    (id, token, uri): (&str, String, &str),
    signers: &Signers,
    idxs: Vec<u64>,
    expect: Vec<u8>,
) -> Value {
    let key = signers.public_key(id);
    let alg = key.algorithm().jose_alg();
    json!({
        "name": name, "alg": alg, "jwk": key.jwk(), "token": token, "uri": uri,
        "idxs": idxs, "expect": expect, "accepts": null,
    })
}

/// Every width under every algorithm, the draft's own example token, and the ways a
/// token goes wrong, with what this crate says of each filled in.
fn corpus(signers: &Signers) -> Vec<Value> {
    let mut cases = Vec::new();

    for (id, _) in KEYS {
        for bits in [1u8, 2, 4, 8] {
            let mut list = TokenStatusList::new(bits, ENTRIES).unwrap();
            let values: Vec<u8> = (0..ENTRIES)
                .map(|idx| u8::try_from((idx * 7 + u64::from(bits)) % (1 << bits)).unwrap())
                .collect();
            for (idx, value) in (0u64..).zip(&values) {
                list.set(idx, *value).unwrap();
            }
            let token = signers.token(id, Some(TYP), &claims(&list));
            cases.push(case(
                &format!("{id} {bits}-bit, every value"),
                (id, token, URI),
                signers,
                (0..ENTRIES).collect(),
                values,
            ));
        }
    }

    // The claims and the list of section 5.1's and 4.2's examples, signed by our key.
    let draft = json!({
        "sub": URI, "iat": ISSUED_AT, "exp": 2_291_720_170u64, "ttl": 43_200,
        "status_list": { "bits": 1, "lst": "eNrbuRgAAhcBXQ" },
    });
    cases.push(case(
        "the draft's example list",
        ("es256", signers.token("es256", Some(TYP), &draft), URI),
        signers,
        (0..16).collect(),
        vec![1, 0, 0, 1, 1, 1, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1],
    ));

    // Cases this crate refuses. What they would read as does not matter, so each
    // asks for the entries it names and expects zero of them.
    let list = small_list();
    let base = claims(&list);
    let bad = |name: &str, typ: Option<&str>, claims: &Value, uri: &str, idxs: Vec<u64>| {
        let token = signers.token("es256", typ, claims);
        let expect = vec![0; idxs.len()];
        case(name, ("es256", token, uri), signers, idxs, expect)
    };
    let plain = |name: &str, claims: &Value| bad(name, Some(TYP), claims, URI, vec![0]);
    let lst = |text: String| {
        with(
            base.clone(),
            "status_list",
            Some(json!({ "bits": 2, "lst": text })),
        )
    };

    cases.extend([
        bad("typ is JWT", Some("JWT"), &base, URI, vec![0]),
        bad("no typ", None, &base, URI, vec![0]),
        bad(
            "a list for another uri",
            Some(TYP),
            &base,
            "https://status.example/statuslists/2",
            vec![0],
        ),
        bad("an index past the list", Some(TYP), &base, URI, vec![16]),
        plain(
            "expired a second ago",
            &with(base.clone(), "exp", Some(json!(NOW - 1))),
        ),
        plain(
            "exp is text",
            &with(base.clone(), "exp", Some(json!("2291720170"))),
        ),
        plain("no sub", &with(base.clone(), "sub", None)),
        plain("no iat", &with(base.clone(), "iat", None)),
        plain("ttl is zero", &with(base.clone(), "ttl", Some(json!(0)))),
        plain(
            "ttl is negative",
            &with(base.clone(), "ttl", Some(json!(-1))),
        ),
        plain("no status_list", &with(base.clone(), "status_list", None)),
        plain(
            "bits is 3",
            &with(
                base.clone(),
                "status_list",
                Some(json!({ "bits": 3, "lst": list.encode() })),
            ),
        ),
        plain(
            "bits is 0",
            &with(
                base.clone(),
                "status_list",
                Some(json!({ "bits": 0, "lst": list.encode() })),
            ),
        ),
        plain("lst is padded", &lst(format!("{}==", list.encode()))),
        plain("lst is gzip", &lst(B64.encode(GZIP_OF_ONE_ZERO_BYTE))),
        plain("lst is not zlib", &lst(B64.encode(b"this is not zlib"))),
    ]);

    // Signed by a key that is not the one the corpus names.
    let forged = signers.token(OTHER_KEY, Some(TYP), &base);
    cases.push(case(
        "signed by another key",
        ("es256", forged, URI),
        signers,
        vec![0],
        vec![0],
    ));

    for entry in &mut cases {
        let accepts = rust_accepts(entry);
        entry["accepts"] = json!(accepts);
    }
    cases
}

/// Whether this crate accepts a case: the token verifies under its key, and every
/// entry asked about reads as a status or as one nobody has registered.
fn rust_accepts(case: &Value) -> bool {
    let Some(key) = PublicKey::from_jwk(&case["jwk"]) else {
        return false;
    };
    let Ok(token) = StatusListToken::verify(case["token"].as_str().unwrap(), &key, now()) else {
        return false;
    };
    case["idxs"].as_array().unwrap().iter().all(|idx| {
        let reference = StatusReference {
            idx: idx.as_u64().unwrap(),
            uri: case["uri"].as_str().unwrap().to_owned(),
        };
        matches!(
            token.status(&reference, now()),
            Ok(_) | Err(TokenStatusListError::UnregisteredStatus(_))
        )
    })
}

#[test]
fn every_case_reads_the_way_the_corpus_says() {
    let signers = Signers::new();
    let cases = corpus(&signers);
    let accepted = cases.iter().filter(|c| c["accepts"] == json!(true)).count();

    // Four widths under four algorithms and the draft's example are read; the rest
    // are refused, each for the one thing it gets wrong.
    assert_eq!(accepted, 17, "the cases this crate accepts");
    assert_eq!(cases.len() - accepted, 17, "the cases this crate refuses");
    for case in &cases {
        let accepts = case["accepts"] == json!(true);
        let name = case["name"].as_str().unwrap();
        let refused_by_design =
            !(name.contains("-bit, every value") || name.starts_with("the draft"));
        assert_eq!(accepts, !refused_by_design, "{name}");
    }

    if std::env::var_os("EMIT_STATUS_LIST_CORPUS").is_none() {
        return;
    }
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/status-list-oracle");
    std::fs::create_dir_all(&dir).expect("create the oracle output directory");
    let body = json!({ "now": NOW, "cases": cases });
    std::fs::write(
        dir.join("corpus.json"),
        serde_json::to_string(&body).expect("serialises"),
    )
    .expect("write the corpus");
    eprintln!("wrote {} tokens to {}", cases.len(), dir.display());
}

/// Where a file of tokens issued by the library and this crate disagree, one line each.
fn disagreements(file: &Value) -> Vec<String> {
    let at = Utc
        .timestamp_opt(file["now"].as_i64().expect("a time"), 0)
        .unwrap();
    let mut found = Vec::new();
    for case in file["cases"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("a name");
        let Some(key) = PublicKey::from_jwk(&case["jwk"]) else {
            found.push(format!("{name}: its key is not one this crate reads"));
            continue;
        };
        let token = match StatusListToken::verify(case["token"].as_str().unwrap(), &key, at) {
            Ok(token) => token,
            Err(e) => {
                found.push(format!("{name}: {e}"));
                continue;
            }
        };
        let uri = case["uri"].as_str().unwrap();
        let entries = case["entries"].as_u64().unwrap();
        for pair in case["expect"].as_array().unwrap() {
            let (idx, value) = (
                pair[0].as_u64().unwrap(),
                u8::try_from(pair[1].as_u64().unwrap()).unwrap(),
            );
            let reference = StatusReference {
                idx,
                uri: uri.to_owned(),
            };
            let read = token.status(&reference, at);
            let agrees = match Status::from_value(value) {
                Ok(status) => read == Ok(status),
                Err(unregistered) => read == Err(unregistered),
            };
            if !agrees {
                found.push(format!(
                    "{name}: entry {idx} is {value} for the library, {read:?} here"
                ));
            }
        }
        let past = StatusReference {
            idx: entries,
            uri: uri.to_owned(),
        };
        let expected = Err(TokenStatusListError::IndexOutOfBounds {
            idx: entries,
            entries,
        });
        if token.status(&past, at) != expected {
            found.push(format!(
                "{name}: the list does not hold {entries} entries here"
            ));
        }
    }
    found
}

#[test]
fn what_an_independent_implementation_issued_verifies_here() {
    let Some(given) = std::env::var_os("CHECK_STATUS_LIST_ORACLE_ISSUED") else {
        return;
    };
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(given);
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let file: Value = serde_json::from_str(&text).expect("the file is JSON");

    let count = file["cases"].as_array().map_or(0, Vec::len);
    assert!(count >= 20, "only {count} tokens to verify");
    let found = disagreements(&file);
    assert!(
        found.is_empty(),
        "{} of {count} tokens issued by an independent implementation are not read the way \
         it wrote them:\n{}",
        found.len(),
        found.join("\n")
    );
    eprintln!("verified {count} tokens issued by an independent implementation");
}

/// A check that has never failed is not known to be one: a file whose expectation is
/// wrong, and one whose token is not signed by its key, must both be reported.
#[test]
fn the_check_fails_on_a_mismatch() {
    let signers = Signers::new();
    let list = small_list();
    let token = signers.token("es256", Some(TYP), &claims(&list));
    let key = signers.public_key("es256");
    let case = |expect: Value, jwk: Value| {
        json!({
            "now": NOW,
            "cases": [{ "name": "c", "jwk": jwk, "token": token, "uri": URI, "entries": 16, "expect": expect }],
        })
    };

    assert!(disagreements(&case(json!([[0, 0], [1, 1], [2, 2], [3, 3]]), key.jwk())).is_empty());
    assert_eq!(
        disagreements(&case(json!([[1, 0]]), key.jwk())).len(),
        1,
        "a wrong value"
    );
    assert_eq!(
        disagreements(&case(json!([[0, 0]]), signers.public_key("eddsa").jwk())).len(),
        1,
        "a wrong key"
    );
}
