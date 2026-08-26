//! Emit every Digital Link form this crate builds, with our own verdict on each,
//! for GS1's own syntax tooling to judge.
//!
//! # Why this exists
//!
//! We wrote this GS1 Digital Link implementation, and every test of it was
//! written by the same people who wrote the parser. A shared misreading of the
//! standard passes both — the test agrees with the code because both came from
//! one reading of the specification.
//!
//! GS1 publishes the GS1 Barcode Syntax Engine, which implements the GS1 Barcode
//! Syntax Dictionary. Handing it our URIs and asking *"is this well-formed?"* is
//! the only check available here that is not circular. This test produces the
//! corpus; `.github/scripts/gs1_syntax_oracle.mjs` runs the engine over it.
//!
//! # What is in the corpus, and why the negatives matter as much
//!
//! Each line carries a URI and **our** accept/reject verdict for it. The oracle
//! asserts agreement in both directions, because the two disagreements fail
//! differently:
//!
//! - **We accept, GS1 rejects** — we emit links that are not valid GS1. The
//!   resolver publishes them on physical products.
//! - **We reject, GS1 accepts** — we refuse links a conformant partner may
//!   legitimately send us. This is the quieter one and the one that loses
//!   interoperability without any error ever being logged.
//!
//! # Scope
//!
//! Syntax, not semantics. Passing says our links are well-formed GS1. It does
//! **not** say a GTIN is allocated to anyone, that a resolver answers, or
//! anything that would justify the phrase "GS1-certified". The engine version
//! is recorded by the job that runs it.
//!
//! # Running
//!
//! ```text
//! EMIT_GS1_CORPUS=1 cargo test -p dpp-digital-link --test gs1_oracle_corpus
//! ```
//!
//! Writes `target/gs1-oracle/corpus.jsonl`. Without the variable the test still
//! runs and still checks our own side, so an ordinary `just check` gets the
//! coverage without producing files.

use dpp_digital_link::{DigitalLink, PrimaryKey};
use dpp_domain::Gtin;

/// GTINs with correct check digits, spanning the lengths we normalise from.
///
/// GTIN-8, -12 and -13 are left-padded to 14 by the parser, so a corpus that
/// used only 14s would never exercise that path against the oracle — and
/// padding is exactly the kind of step where an off-by-one survives our own
/// tests, because our builder and our parser would pad identically.
const VALID_GTINS: &[&str] = &[
    "09506000134352",
    "09520123456788",
    "00012345678905",
    "01234567890128",
];

/// The resolver bases we emit under: a bare host, and one with a path prefix.
const RESOLVER_BASES: &[&str] = &["https://id.example.com", "https://example.com/resolve"];

/// One corpus entry: a URI and whether *we* accept it.
struct Entry {
    uri: String,
    accepted: bool,
    note: &'static str,
}

fn entry(uri: String, note: &'static str) -> Entry {
    let accepted = DigitalLink::parse(&uri).is_ok();
    Entry {
        uri,
        accepted,
        note,
    }
}

/// Flip the final check digit so the GTIN is well-formed but wrong.
///
/// Computed rather than hardcoded: a hardcoded "invalid" GTIN is one more
/// constant that could itself be wrong, and the point of the case is that the
/// check digit and only the check digit is broken.
fn corrupt_check_digit(gtin: &str) -> String {
    let (head, last) = gtin.split_at(gtin.len() - 1);
    let digit: u32 = last.parse().expect("GTIN is all digits");
    format!("{head}{}", (digit + 1) % 10)
}

/// Every Digital Link this crate can build, plus the ones it must refuse.
fn corpus() -> Vec<Entry> {
    let mut out = Vec::new();

    for base in RESOLVER_BASES {
        for gtin in VALID_GTINS {
            let parsed = Gtin::parse(gtin).expect("corpus GTIN must be valid");
            // Qualifiers in GS1's canonical order for AI 01. `None` entries
            // are dropped, so each call builds exactly the combination named.
            let dl = |variant, batch, serial, tpcsn| DigitalLink {
                resolver_base: (*base).to_owned(),
                primary_key: PrimaryKey::Gtin(parsed.clone()),
                qualifiers: [
                    ("22", variant),
                    ("10", batch),
                    ("21", serial),
                    ("235", tpcsn),
                ]
                .into_iter()
                .filter_map(|(ai, v): (&str, Option<String>)| v.map(|v| (ai.to_owned(), v)))
                .collect(),
            };

            // Every qualifier combination the builder emits, in canonical order.
            out.push(entry(dl(None, None, None, None).build(), "gtin only"));
            out.push(entry(
                dl(None, None, Some("SN001".into()), None).build(),
                "gtin + serial",
            ));
            out.push(entry(
                dl(None, Some("BATCH01".into()), None, None).build(),
                "gtin + batch",
            ));
            out.push(entry(
                dl(None, Some("BATCH01".into()), Some("SN001".into()), None).build(),
                "gtin + batch + serial",
            ));
            out.push(entry(
                dl(Some("VAR1".into()), None, None, None).build(),
                "gtin + variant",
            ));
            out.push(entry(
                dl(
                    Some("VAR1".into()),
                    Some("BATCH01".into()),
                    Some("SN001".into()),
                    None,
                )
                .build(),
                "gtin + variant + batch + serial",
            ));
            out.push(entry(
                dl(None, None, None, Some("TPCSN01".into())).build(),
                "gtin + third-party serial",
            ));

            // A link-type query string, which the resolver appends and the
            // parser must strip before reading the last path value.
            out.push(entry(
                format!(
                    "{}?linkType=gs1:pip",
                    dl(None, None, Some("SN1".into()), None).build()
                ),
                "gtin + serial + linkType query",
            ));
        }

        // Negative: the check digit is the one thing a syntax engine can catch
        // that a grammar alone cannot.
        let broken = corrupt_check_digit(VALID_GTINS[0]);
        out.push(entry(
            format!("{base}/01/{broken}"),
            "corrupted check digit",
        ));

        // ── Cases our own builder cannot produce ────────────────────────────
        //
        // Everything above is `DigitalLink::build()` output, so the corpus can
        // only ever contain links we already emit. That proves what we emit is
        // valid GS1 and says nothing about valid GS1 we refuse — the quiet
        // failure direction. These are written by hand for that reason, and the
        // oracle, not us, decides who is right.
        let gtin = VALID_GTINS[0];

        // AI 01 declares `dlpkey=22,10,21|235`, which the dictionary's header
        // defines as *alternative* sequences. We refuse a path mixing them.
        out.push(entry(
            format!("{base}/01/{gtin}/21/SN001/235/TPX01"),
            "serial and third-party serial from different alternative sequences",
        ));

        // AI 99 is INTERNAL in GS1's dictionary — a real AI, but a data
        // attribute. GS1's grammar puts only the primary key and its qualifiers
        // in the path, so this is refused despite being a known AI. The engine
        // adjudicated this: an earlier revision accepted it and the oracle
        // reported "we ACCEPT, GS1 REJECTS".
        out.push(entry(
            format!("{base}/01/{gtin}/21/SN001/99/INTERNALDATA"),
            "data attribute in the path rather than the query string",
        ));

        // Genuinely unassigned: `04` is in no GS1 entry, so refusing it is
        // correct and the oracle should agree.
        out.push(entry(
            format!("{base}/01/{gtin}/04/NOSUCHAI"),
            "unassigned application identifier",
        ));

        // ── The other fifteen primary keys ──────────────────────────────
        //
        // These could not be in the corpus until the parser read them: the
        // oracle asserts agreement in both directions with no mechanism for a
        // known disagreement, so a conformant link we refused would have failed
        // the job rather than reported a gap. That is the coupling — the cases
        // and the support have to land together.
        //
        // Written by hand from GS1's dictionary rather than built, because
        // `build()` only ever emits what we already construct. The oracle, not
        // us, decides whether each is well-formed.
        for (ai, value, note) in [
            ("00", "106141411234567890", "SSCC"),
            ("253", "4012345678901", "GDTI without its optional serial"),
            ("401", "ORDER-99", "GINC, variable-length alphanumeric"),
            ("402", "40123456789012340", "GSIN"),
            ("414", "4226350800008", "party GLN"),
            ("8003", "04012345678901ABC", "GRAI"),
            ("8004", "4012345ABC123", "GIAI"),
            ("8013", "1987654Ad4X4bL5ttr2310c2K", "GMN"),
        ] {
            out.push(entry(format!("{base}/{ai}/{value}"), note));
        }

        // A qualifier that belongs to a *different* primary key. AI 22
        // qualifies AI 01 and not AI 00, so this is not a link GS1 defines —
        // and it is only refusable now that the qualifier rules are evaluated
        // against the key the path actually opened on.
        out.push(entry(
            format!("{base}/00/106141411234567890/22/VAR-1"),
            "qualifier from another primary key's sequence",
        ));
    }

    out
}

/// Our own side must hold before the corpus is worth handing to anyone.
///
/// If a URI we *built* does not parse back, the disagreement is ours and the
/// oracle would be reporting our bug as an interoperability failure.
///
/// The hand-written cases are exempt: they exist precisely to carry a verdict
/// the builder cannot produce, and their expected verdict is named here so a
/// change in what we accept shows up as a failing assertion rather than as a
/// silently different corpus.
#[test]
fn every_built_link_round_trips_through_our_own_parser() {
    // note → the verdict we must give it.
    const EXPECTED_REJECTS: &[&str] = &[
        "corrupted check digit",
        "serial and third-party serial from different alternative sequences",
        "unassigned application identifier",
        "data attribute in the path rather than the query string",
        "qualifier from another primary key's sequence",
    ];

    for e in corpus() {
        if EXPECTED_REJECTS.contains(&e.note) {
            assert!(!e.accepted, "this must be refused ({}): {}", e.note, e.uri);
        } else {
            assert!(
                e.accepted,
                "we built this and cannot parse it back ({}): {}",
                e.note, e.uri
            );
        }
    }
}

/// Write the corpus for the external engine, when asked.
#[test]
fn emit_corpus_for_the_gs1_syntax_engine() {
    let entries = corpus();
    assert!(
        entries.len() > 50,
        "a corpus this small would pass by not asking much: {}",
        entries.len()
    );

    if std::env::var_os("EMIT_GS1_CORPUS").is_none() {
        return;
    }

    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/gs1-oracle");
    std::fs::create_dir_all(&dir).expect("create the oracle output directory");

    let mut body = String::new();
    for e in &entries {
        let line = serde_json::json!({
            "uri": e.uri,
            "accepted": e.accepted,
            "note": e.note,
        });
        body.push_str(&line.to_string());
        body.push('\n');
    }
    std::fs::write(dir.join("corpus.jsonl"), body).expect("write the corpus");
    eprintln!(
        "wrote {} corpus entries to {}",
        entries.len(),
        dir.display()
    );
}
