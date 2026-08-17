use super::*;
use crate::vocabulary::{Layer, Status};

fn register() -> VocabularyRegister {
    VocabularyRegister::new()
}

#[test]
fn every_embedded_record_parses_and_its_key_matches_its_file() {
    // `new()` asserts both. This test exists so the failure surfaces as a named
    // test rather than as a panic inside whatever ran first.
    let r = register();
    assert!(!r.all().is_empty(), "register loaded no vocabularies");
}

#[test]
fn keys_are_unique() {
    let r = register();
    let mut keys: Vec<&str> = r.all().iter().map(|v| v.key.as_str()).collect();
    keys.sort_unstable();
    let before = keys.len();
    keys.dedup();
    assert_eq!(
        before,
        keys.len(),
        "duplicate vocabulary key in the register"
    );
}

#[test]
fn every_record_carries_a_finding_and_a_next_step() {
    // A record whose `finding` is empty has recorded that a vocabulary exists
    // and nothing else, which is the state this crate exists to make visible.
    for v in register().all() {
        assert!(
            !v.finding.trim().is_empty(),
            "vocabulary '{}' has no finding",
            v.key
        );
        assert!(
            !v.next_step.trim().is_empty(),
            "vocabulary '{}' has no next step",
            v.key
        );
        assert!(
            !v.checked_on.trim().is_empty(),
            "vocabulary '{}' has no checkedOn date",
            v.key
        );
    }
}

#[test]
fn our_own_namespace_is_always_permitted() {
    let r = register();
    assert_eq!(
        r.verdict("urn:odal-node:aas:submodel-template:battery-technical-data:1.0"),
        Verdict::Own
    );
    assert!(r.verdict("urn:odal-node:anything:at:all").is_permitted());
}

#[test]
fn an_unrecorded_iri_is_refused_as_unknown() {
    // The most serious verdict: an identifier reached a serialisation without
    // anyone recording where it came from.
    let v = register().verdict("https://example.invalid/voc/madeUp");
    assert_eq!(v, Verdict::Unknown);
    assert!(!v.is_permitted());
    assert!(v.reason().contains("unrecorded"));
}

#[test]
fn the_six_removed_idta_and_eclass_identifiers_stay_refused() {
    // These were emitted by a published crate and removed in 0.13.0 because
    // they were wrong, not stale. If any of them ever becomes permitted, it
    // must be because a person read the specification — never because a record
    // drifted.
    let r = register();
    for iri in [
        "https://admin-shell.io/IDTA/02006/2/0",
        "https://admin-shell.io/IDTA/02004/1/2/Manufacturer",
        "https://admin-shell.io/IDTA/02023/0/9",
        "https://admin-shell.io/IDTA/02011/1/0",
        "urn:eclass:0173-1#01-AKJ975#001",
        "urn:idta:aas:submodel:digital-product-passport:1.0",
    ] {
        assert!(
            !r.verdict(iri).is_permitted(),
            "'{iri}' is permitted; it was removed from a published crate for being wrong"
        );
    }
}

#[test]
fn a_conformance_bridge_is_never_speakable_even_when_verified() {
    // The Catena-X identifier is the one third-party identifier this project
    // has ever checked and found clean — correct form, current release, stated
    // licence. It is still refused, because being measured against a model is
    // not the same as speaking its language.
    let r = register();
    let v = r.verdict("urn:samm:io.catenax.battery.battery_pass:6.0.0#BatteryPass");
    assert_eq!(
        v,
        Verdict::BridgeNotSpeakable {
            vocabulary: "catena-x-battery-pass".to_owned()
        }
    );
    assert!(v.reason().contains("tested by it"));
}

#[test]
fn batterypass_consortium_bridge_is_never_speakable_even_when_verified() {
    // Read 2026-08-11: the model files' own SPDX headers confirm CC-BY-4.0
    // (the surveyed `licence: null` was wrong), and the field-by-field
    // comparison against `BatteryData` is recorded in the vocabulary's own
    // `finding`. Same shape as the Catena-X case above — verified as a
    // conformance target, still refused as a namespace to speak.
    let r = register();
    let v = r.verdict("urn:samm:io.BatteryPass:1.2.0#GeneralProductInformation");
    assert_eq!(
        v,
        Verdict::BridgeNotSpeakable {
            vocabulary: "batterypass-samm".to_owned()
        }
    );
    assert!(v.reason().contains("tested by it"));
}

#[test]
fn exactly_six_vocabularies_are_verified_so_far() {
    // `batterypass-samm`, `catena-x-battery-pass`, `ec-battery-guidance`,
    // `gs1`, `idta` and `semic` — all read 2026-08-11, see each one's own
    // `finding` for who and what. Everything else stays `Surveyed`/`Tracked`
    // until someone reads it too. When this count changes, update it here
    // and record the reader in the vocabulary's own `checkedOn`/`finding`
    // fields, not just in this test.
    let r = register();
    let mut verified: Vec<&str> = r
        .all()
        .iter()
        .filter(|v| v.status == Status::Verified)
        .map(|v| v.key.as_str())
        .collect();
    verified.sort_unstable();
    assert_eq!(
        verified,
        vec![
            "batterypass-samm",
            "catena-x-battery-pass",
            "ec-battery-guidance",
            "gs1",
            "idta",
            "semic"
        ]
    );
}

#[test]
fn an_iri_minted_by_an_intermediary_is_not_recorded_as_the_authoritys_namespace() {
    // Two entries describe vocabularies whose IRIs exist only as a third
    // party's coinage — the Commission publishes none for its battery guidance,
    // and GEFEG's attributes are served under the surveying site's own prefix.
    // Recording those as the authority's namespace would assert the authority's
    // backing for an identifier it never minted.
    let r = register();
    for key in ["ec-battery-guidance", "batterypass-ready"] {
        let v = r.get(key).expect("record present");
        assert!(
            v.namespace_iri.is_none(),
            "'{key}' carries a namespace IRI; the authority publishes none, so any \
             value here belongs to an intermediary"
        );
    }
    // And nothing under the surveying site's prefix resolves to them.
    assert_eq!(
        r.verdict("https://ref.openepcis.io/vocab/ec-battery-passport-guidance/1.0#dataPoint"),
        Verdict::Unknown
    );
}

#[test]
fn http_and_https_forms_of_a_namespace_are_different_namespaces() {
    // SEMIC mints on `http`. An IRI is compared as a string, so silently
    // upgrading one matches an IRI nobody publishes.
    let r = register();
    assert!(
        r.vocabulary_for("http://data.europa.eu/m8g/Evidence")
            .is_some()
    );
    assert!(
        r.vocabulary_for("https://data.europa.eu/m8g/Evidence")
            .is_none()
    );
}

#[test]
fn longest_prefix_wins_when_namespaces_nest() {
    // Not a hypothetical: GS1 publishes both `ref.gs1.org/voc/` and sectoral
    // vocabularies elsewhere, and a future entry could nest.
    let r = register();
    let hit = r
        .vocabulary_for("https://ref.gs1.org/voc/gtin")
        .expect("gs1 namespace matches");
    assert_eq!(hit.key, "gs1");
}

#[test]
fn layers_are_distributed_as_the_survey_describes() {
    // A drift guard on the classification itself: if every vocabulary ends up
    // in one layer, the layer distinction has stopped carrying information.
    let r = register();
    let bridges = r
        .all()
        .iter()
        .filter(|v| v.layer == Layer::ConformanceBridge)
        .count();
    let foundational = r
        .all()
        .iter()
        .filter(|v| v.layer == Layer::Foundational)
        .count();
    assert!(bridges >= 2, "expected several conformance bridges");
    assert!(
        foundational >= 2,
        "expected several foundational vocabularies"
    );
}

#[test]
fn a_licence_is_recorded_or_explicitly_unknown() {
    // `None` means not established, and blocks reuse. This asserts the field is
    // never an empty string pretending to be an answer.
    for v in register().all() {
        if let Some(licence) = &v.licence {
            assert!(
                !licence.trim().is_empty(),
                "vocabulary '{}' has an empty licence string; use null for \
                 'not established'",
                v.key
            );
        }
    }
}

/// A claim a record has retracted must not survive anywhere in the crate's prose.
///
/// # Why this exists
///
/// The register's whole purpose is that a fact about someone else's publication
/// has **one home**, with a source and a `checkedOn` date. Prose that restates
/// such a fact is a copy, and a copy is what drifts: this crate shipped a README
/// paragraph asserting the EN 182xx series carried no OJEU citation, at the same
/// version as a `jtc24` record whose own `finding` calls that claim stale and
/// cites the Implementing Decision that made it so. Both went to crates.io
/// together.
///
/// Nothing in the existing gate could catch it. `cargo doc` compiles the README
/// as a doctest but does not read it; the register tests check the records, not
/// the prose about them.
///
/// # How to use it
///
/// When a record's `finding` retracts a claim, add the retracted phrasing here
/// with the record that killed it. This is the corrections-register discipline
/// the project already applies to regulatory claims, pointed at our own writing.
///
/// # What it deliberately is not
///
/// A general prose checker. Phrases are listed one at a time, on purpose:
/// anything fuzzy enough to catch a paraphrase is fuzzy enough to fire on
/// innocent text, and a tripwire that cries wolf is one people switch off. It
/// catches the *exact* claim coming back — which is the failure that actually
/// happened, twice now, by copy rather than by reinvention.
#[test]
fn a_retracted_claim_does_not_reappear_in_the_crates_prose() {
    /// `(retracted phrasing, the record whose finding retracted it, why)`.
    const RETRACTED: &[(&str, &str, &str)] = &[(
        "no OJEU citation",
        "jtc24",
        "six EN 182xx:2026 standards are cited as harmonised — see the record's \
         source and finding, which name the Implementing Decision",
    )];

    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut prose: Vec<(String, String)> = Vec::new();

    let readme = crate_root.join("README.md");
    prose.push((
        "README.md".to_owned(),
        std::fs::read_to_string(&readme).expect("README.md is readable"),
    ));

    // Doc comments count too: the three recorded instances of this drift class
    // were a module doc, a schema description and a README paragraph.
    let src = crate_root.join("src");
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("src is readable") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&path).expect("source is readable");
                prose.push((path.display().to_string(), text));
            }
        }
    }

    let mut offenders = Vec::new();
    for (phrase, record, why) in RETRACTED {
        for (name, text) in &prose {
            // This test necessarily contains every phrase it forbids.
            if name.ends_with("tests.rs") {
                continue;
            }
            if text.contains(phrase) {
                offenders.push(format!(
                    "{name} still asserts {phrase:?}, which the '{record}' record retracted: {why}"
                ));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "retracted claims found in this crate's prose — reference the record instead of \
         restating it:\n  {}",
        offenders.join("\n  ")
    );
}

/// The record that retraction rests on is still present and still says so.
///
/// Without this, the guard above passes trivially the day someone edits the
/// `jtc24` finding: the phrase would be gone from the prose *and* the reason for
/// forbidding it would be gone from the register, and nothing would say the pair
/// had come apart.
#[test]
fn the_record_behind_the_retraction_still_carries_it() {
    let register = register();
    let jtc24 = register
        .all()
        .iter()
        .find(|r| r.key == "jtc24")
        .expect("the jtc24 record exists")
        .clone();

    assert!(
        jtc24.finding.contains("harmonised"),
        "the jtc24 finding must still record the harmonisation that retracted the \
         'no OJEU citation' claim; if this changed, revisit the retraction list"
    );
    assert!(
        jtc24.source.contains("2026/1736"),
        "and it must still cite the Implementing Decision it was verified against"
    );
}
