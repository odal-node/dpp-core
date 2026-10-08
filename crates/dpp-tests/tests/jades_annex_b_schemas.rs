//! The JAdES protected header this workspace builds, held to ETSI's own JSON
//! Schemas.
//!
//! # Where the schemas come from
//!
//! Annex B of ETSI TS 119 182-1 V1.2.1 is normative, and names a set of JSON
//! Schema files for checking a JAdES signature's structure. The one used here is
//! `19182-protected-jsonSchema.json`, which "may be used by implementers to
//! validate the conformance of the JWS Protected Header of a JAdES signature". It
//! refers to the other files by relative path, so all of them are vendored and
//! served to the validator from memory. See `fixtures/jades/NOTICE.md`.
//!
//! This is evidence of a different kind from the Rust tests of the module, which
//! check the output against this repository's own reading of the standard, and
//! from `jades-oracle.yml`, which asks an independent implementation to judge one
//! signature. Here the standard's own schema judges every header shape the
//! builder can emit, offline and on every run.
//!
//! # What this cannot prove
//!
//! - **The schema is weaker than the standard.** It encodes clause 5.1.7's rule
//!   that a signature carries *at least one of* `x5t#S256`, `x5c`, `sigX5ts` and
//!   `x5t#o`, so it accepts a header with `x5c` alone. DSS does not call such a
//!   signature baseline, and reads Table 1's certificate-reference service as
//!   needing a digest form. `the_schema_accepts_x5c_alone_which_dss_does_not`
//!   asserts the gap, so it is stated and not merely believed. The two checks
//!   together are stronger than either.
//! - **The published schema cannot be applied as written.** It declares
//!   `contentEncoding: base64` for `x5t#S256`, which RFC 7515 clause 4.1.8 defines
//!   as base64url, and a validator that asserts the keyword as standard base64
//!   rejects a correctly encoded thumbprint containing `-` or `_`. This file reads
//!   `base64` as either alphabet, padded or not. That is stricter than ignoring the
//!   keyword, since a string that is not base64 in either alphabet still fails,
//!   and no stricter than the RFCs.
//!   `the_schema_as_published_rejects_a_correctly_encoded_thumbprint` asserts the
//!   defect, so the reading is a recorded choice and not a quiet one.
//! - **It judges the protected header and nothing else.** Not the signature value,
//!   not the certificate, not trust. A header that passes can still sit on a
//!   signature that does not verify.
//! - **The header's key order and the dummy certificate are not what is under
//!   test.** The certificate here is a few arbitrary bytes, because the schema
//!   checks that `x5c` entries are strings and not what is inside them.
//!   `jades-oracle.yml` judges a real one.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use base64::Engine;
use dpp_crypto::jades::{CertificateRef, JadesHeader, prepare};
use jsonschema::{Retrieve, Uri};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/jades");

/// Where the schemas' relative references resolve. It is the location ETSI
/// publishes them at for V1.2.1, and nothing is fetched from it.
const BASE: &str = "https://forge.etsi.org/rep/esi/x19_182_JAdES/raw/v1.2.1/";

const B64URL: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// Every file under `dir`, as a sorted `/`-separated path relative to it.
fn files_under(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
        let entries =
            fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot scan {}: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                let relative = path.strip_prefix(root).expect("under the root");
                out.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// Serves the vendored schema files by URI, so a relative `$ref` resolves
/// without the workspace's `jsonschema` build having any way to fetch.
struct Vendored(Arc<HashMap<String, Value>>);

impl Retrieve for Vendored {
    fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        self.0
            .get(uri.as_str())
            .cloned()
            .ok_or_else(|| format!("no vendored schema at {uri}").into())
    }
}

/// Whether `text` is base64 in either alphabet, padded or not.
///
/// What ETSI's schemas mean by `contentEncoding: base64`. See the module docs: the
/// published schema says `base64` of fields that RFC 7515 defines as base64url.
fn is_base64_in_either_alphabet(text: &str) -> bool {
    use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
    [STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD]
        .iter()
        .any(|engine| engine.decode(text).is_ok())
}

/// The `jsonschema` hook that pairs with the check above. Only used for a
/// `contentMediaType`, which these schemas never declare.
fn no_conversion(_: &str) -> Result<Option<String>, jsonschema::ValidationError<'static>> {
    Ok(None)
}

/// A validator for ETSI's `19182-protected-jsonSchema.json`, with every file it
/// refers to resolvable, reading `contentEncoding: base64` as either alphabet.
fn protected_header_validator() -> jsonschema::Validator {
    build_protected_header_validator(true)
}

/// The same, with `contentEncoding: base64` read as this crate reads it by
/// default: standard base64 only.
fn protected_header_validator_as_published() -> jsonschema::Validator {
    build_protected_header_validator(false)
}

fn build_protected_header_validator(either_alphabet: bool) -> jsonschema::Validator {
    let root = Path::new(FIXTURES).join("ts-119-182-1-v1.2.1");
    let mut files = HashMap::new();
    for relative in files_under(&root) {
        if !relative.ends_with(".json") {
            continue;
        }
        let text = fs::read_to_string(root.join(&relative))
            .unwrap_or_else(|e| panic!("cannot read {relative}: {e}"));
        let value: Value =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{relative} is not JSON: {e}"));
        files.insert(format!("{BASE}{relative}"), value);
    }
    let entry = format!("{BASE}19182-protected-jsonSchema.json");
    let schema = files
        .get(&entry)
        .cloned()
        .expect("the protected-header schema is vendored");
    let mut options = jsonschema::draft7::options()
        .with_base_uri(entry)
        .with_retriever(Vendored(Arc::new(files)));
    if either_alphabet {
        options =
            options.with_content_encoding("base64", is_base64_in_either_alphabet, no_conversion);
    }
    options
        .build(&schema)
        .expect("ETSI's protected-header schema compiles with its references resolved")
}

/// The protected header segment of a fully assembled compact signature,
/// decoded: the bytes the signature covers, as a verifier receives them.
fn protected_header_of(header: &JadesHeader) -> Value {
    let compact = prepare(header, br#"{"passportId":"p"}"#)
        .expect("prepares")
        .assemble(&[0u8; 64])
        .into_string();
    let segment = compact
        .split('.')
        .next()
        .expect("a compact JWS has segments");
    serde_json::from_slice(&B64URL.decode(segment).expect("the header is base64url"))
        .expect("the header is JSON")
}

/// Arbitrary bytes standing in for a certificate. See the module docs.
fn dummy_der() -> Vec<u8> {
    vec![0x30, 0x03, 0x02, 0x01, 0x00]
}

fn header_with(certificate: CertificateRef, content_type: Option<&str>) -> JadesHeader {
    JadesHeader {
        alg: "EdDSA".into(),
        iat: 1_800_000_000,
        certificate,
        content_type: content_type.map(str::to_owned),
    }
}

fn errors_of(validator: &jsonschema::Validator, header: &Value) -> Vec<String> {
    validator
        .iter_errors(header)
        .map(|e| format!("at `{}`: {e}", e.instance_path()))
        .collect()
}

/// **The claim.** Every header shape the builder is meant to produce at B-B
/// satisfies ETSI's schema: with the certificate chain and its digest together,
/// which is the form the EU profile asks for, and with the digest alone, which is
/// valid JAdES; each with and without a content type.
#[test]
fn the_header_this_crate_builds_satisfies_etsis_schema() {
    let validator = protected_header_validator();
    let der = dummy_der();
    let forms = [
        (
            "x5c with x5t#S256",
            CertificateRef::chain_of_der(std::slice::from_ref(&der)).expect("a chain"),
        ),
        ("x5t#S256 alone", CertificateRef::thumbprint_of_der(&der)),
    ];

    let mut faults = Vec::new();
    for (name, certificate) in forms {
        for content_type in [None, Some("json")] {
            let header = protected_header_of(&header_with(certificate.clone(), content_type));
            for error in errors_of(&validator, &header) {
                faults.push(format!(
                    "{name}, cty {content_type:?}: {error}\n    {header}"
                ));
            }
        }
    }
    assert!(
        faults.is_empty(),
        "headers ETSI's schema rejects:\n{}",
        faults.join("\n")
    );
}

/// The defect in the published schema, asserted. It declares
/// `contentEncoding: base64` for `x5t#S256`, which RFC 7515 clause 4.1.8 defines as
/// base64url. A validator that asserts the keyword as standard base64, as this
/// crate does by default, rejects every correctly encoded thumbprint that contains
/// `-` or `_`, and so rejects a good JAdES header. If this starts failing, ETSI
/// has fixed the schema or the crate has changed what it asserts, and the
/// either-alphabet check in this file and the module docs can go.
#[test]
fn the_schema_as_published_rejects_a_correctly_encoded_thumbprint() {
    // A thumbprint with `-` in it, which is base64url and not standard base64.
    let header = json!({
        "alg": "EdDSA",
        "iat": 1_800_000_000,
        "x5t#S256": "tWCDPW94evRhE7lqrU3VtdGuANzMac8wzJK-1lHFZhc",
    });
    assert!(
        B64URL
            .decode("tWCDPW94evRhE7lqrU3VtdGuANzMac8wzJK-1lHFZhc")
            .is_ok_and(|digest| digest.len() == 32),
        "the thumbprint is a well-formed base64url SHA-256"
    );

    assert!(
        !errors_of(&protected_header_validator_as_published(), &header).is_empty(),
        "the schema as published now accepts base64url, so the either-alphabet check is \
         no longer needed"
    );
    assert!(
        errors_of(&protected_header_validator(), &header).is_empty(),
        "and it is accepted when `base64` is read as either alphabet"
    );
}

/// The gap the module docs name, asserted. ETSI's schema takes clause 5.1.7's
/// "at least one of", so `x5c` alone passes. DSS reports a signature with only
/// `x5c` as not baseline, because Table 1's service for the signing certificate
/// offers the digest forms and not `x5c`. If this starts failing, ETSI has
/// tightened the schema and the docs and the claim should say so.
#[test]
fn the_schema_accepts_x5c_alone_which_dss_does_not() {
    let validator = protected_header_validator();
    let chain_only = CertificateRef::Chain(vec![
        base64::engine::general_purpose::STANDARD.encode(dummy_der()),
    ]);
    let header = protected_header_of(&header_with(chain_only, None));

    assert!(header.get("x5c").is_some() && header.get("x5t#S256").is_none());
    assert!(
        errors_of(&validator, &header).is_empty(),
        "ETSI's schema now rejects an x5c-only header, so it is no longer weaker than \
         the module's reading of Table 1: {header}"
    );
}

/// The check is only worth having if it rejects what it is for, and each case
/// below is one clause of the schema.
#[test]
fn the_schema_rejects_what_it_is_for() {
    let validator = protected_header_validator();
    let good = json!({
        "alg": "EdDSA",
        "iat": 1_800_000_000,
        "x5t#S256": B64URL.encode([7u8; 32]),
    });
    assert!(
        errors_of(&validator, &good).is_empty(),
        "the control header is valid"
    );

    let mut cases: Vec<(&str, Value)> = Vec::new();
    let without = |key: &str| {
        let mut header = good.clone();
        header.as_object_mut().expect("an object").remove(key);
        header
    };
    cases.push(("`alg` is absent", without("alg")));
    cases.push(("no certificate reference at all", without("x5t#S256")));
    cases.push(("no signing time", without("iat")));

    let with = |key: &str, value: Value| {
        let mut header = good.clone();
        header[key] = value;
        header
    };
    cases.push((
        "both `iat` and `sigT`",
        with("sigT", json!("2027-03-01T12:00:00Z")),
    ));
    cases.push((
        "the SHA-1 `x5t` is present",
        with("x5t", json!("AAAAAAAAAAAAAAAAAAAAAAAAAAA")),
    ));
    cases.push(("`alg` is not a string", with("alg", json!(7))));
    cases.push(("`iat` is not a number", with("iat", json!("yesterday"))));
    cases.push(("`x5c` is not an array", with("x5c", json!("MAMCAQA="))));
    cases.push(("`crit` is empty", with("crit", json!([]))));

    for (why, header) in cases {
        assert!(
            !errors_of(&validator, &header).is_empty(),
            "{why}: ETSI's schema must reject {header}"
        );
    }
}

/// The files are evidence only while they are ETSI's bytes. A schema edited until
/// a header passes would otherwise pass.
#[test]
fn the_vendored_files_are_the_bytes_recorded_here() {
    let notice = fs::read_to_string(Path::new(FIXTURES).join("NOTICE.md")).expect("NOTICE.md");
    let unquote = |cell: &str| {
        cell.strip_prefix('`')
            .and_then(|c| c.strip_suffix('`'))
            .map(str::to_owned)
    };

    let mut recorded: HashMap<String, (u64, String)> = HashMap::new();
    for line in notice.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() != 5 {
            continue;
        }
        if let (Some(path), Ok(size), Some(hash)) = (
            unquote(cells[1]),
            cells[2].parse::<u64>(),
            unquote(cells[3]),
        ) {
            recorded.insert(path, (size, hash));
        }
    }
    assert!(
        !recorded.is_empty(),
        "NOTICE.md records no file, so nothing would be checked"
    );

    let mut faults = Vec::new();
    for (path, (size, hash)) in &recorded {
        match fs::read(Path::new(FIXTURES).join(path)) {
            Err(e) => faults.push(format!("{path}: cannot be read: {e}")),
            Ok(bytes) => {
                if bytes.len() as u64 != *size {
                    faults.push(format!(
                        "{path}: {} bytes, NOTICE.md records {size}",
                        bytes.len()
                    ));
                }
                let actual = hex::encode(Sha256::digest(&bytes));
                if actual != *hash {
                    faults.push(format!(
                        "{path}: SHA-256 {actual}, NOTICE.md records {hash}"
                    ));
                }
            }
        }
    }
    for path in files_under(Path::new(FIXTURES)) {
        if path != "NOTICE.md" && !recorded.contains_key(&path) {
            faults.push(format!(
                "{path}: is vendored but NOTICE.md does not record it"
            ));
        }
    }
    assert!(
        faults.is_empty(),
        "vendored files differ from NOTICE.md:\n{}",
        faults.join("\n")
    );
}
