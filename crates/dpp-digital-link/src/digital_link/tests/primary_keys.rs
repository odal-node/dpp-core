//! The GS1 primary keys the dictionary names.

use super::super::*;

/// A link keyed on any of GS1's sixteen primary keys parses and round-trips.
///
/// These are conformant Digital Links a partner may legitimately send. Refusing
/// them as `MissingGtin` claimed the sender's link was malformed, which sends
/// the investigation to the wrong side of the integration — the "we reject, GS1
/// accepts" direction the oracle's own header calls the quiet one.
#[test]
fn every_gs1_primary_key_parses_and_round_trips() {
    // SSCC, GDTI, GRAI, GIAI, the party GLN, GINC, GMN.
    for (ai, value) in [
        ("00", "106141411234567897"),
        ("253", "4012345678901"),
        ("8003", "04012345678901ABC"),
        ("8004", "4012345ABC123"),
        ("414", "4226350800008"),
        ("401", "4012345ORDER99"),
        ("8013", "1987654Ad4X4bL5ttr2310c2K"),
    ] {
        let uri = format!("https://id.odal-node.io/{ai}/{value}");
        let dl = DigitalLink::parse(&uri).unwrap_or_else(|e| panic!("{uri} must parse: {e}"));
        assert_eq!(dl.primary_key.ai(), ai);
        assert_eq!(dl.build(), uri, "{ai} must round-trip");
    }
}

/// The one validated key is reachable as a `Gtin`; the other fifteen are not,
/// and can only be read through a method that says so.
///
/// This is the API carrying the asymmetry rather than a doc paragraph carrying
/// it. Every key's check digit or check character pair is verified on the way
/// in, but only the GTIN comes back as a type this workspace models; the others
/// are strings, and a caller has to say so to read one.
#[test]
fn only_the_gtin_comes_back_validated() {
    let gtin_link = DigitalLink::parse("https://id.odal-node.io/01/09506000134352").unwrap();
    assert!(gtin_link.gtin().is_some(), "a GTIN is validated");
    assert_eq!(
        gtin_link.primary_key.unvalidated_value(),
        None,
        "a validated key must not be reachable through the unvalidated accessor"
    );

    let sscc_link = DigitalLink::parse("https://id.odal-node.io/00/106141411234567897").unwrap();
    assert!(
        sscc_link.gtin().is_none(),
        "an SSCC is not a GTIN and must not masquerade as one"
    );
    assert_eq!(
        sscc_link.primary_key.unvalidated_value(),
        Some("106141411234567897"),
        "the only way to its value names what it is"
    );
}

/// A qualifier belonging to a *different* primary key is refused.
///
/// AI 22 is one of AI 01's qualifiers and not one of AI 00's, so an SSCC path
/// carrying it is not a link GS1 defines. Before the primary key was read from
/// the dictionary the qualifier rules were evaluated against AI 01 whatever the
/// path opened on, which would have accepted this.
#[test]
fn a_qualifier_from_another_primary_key_is_refused() {
    let uri = "https://id.odal-node.io/00/106141411234567897/22/VAR-1";
    assert!(
        DigitalLink::parse(uri).is_err(),
        "AI 22 does not qualify an SSCC"
    );
}

/// The set is read from the vendored dictionary, not from a list in the code,
/// so it tracks GS1's designations rather than a snapshot of them.
///
/// Guards the vacuous case too: if the dictionary ever failed to load, every
/// lookup would return `None`, the loop above would panic, and this would
/// report zero — but a future refactor could make the first silent.
#[test]
fn the_primary_key_set_comes_from_the_dictionary() {
    let keys: Vec<&str> = dictionary()
        .values()
        .filter(|s| s.dl_primary_key)
        .map(|s| s.ai.as_str())
        .collect();

    assert!(
        keys.len() > 1,
        "the dictionary should mark several primary keys, found {}",
        keys.len()
    );
    assert!(keys.contains(&"01"), "GTIN must be among them");
    assert!(keys.contains(&"00"), "SSCC must be among them");
}
