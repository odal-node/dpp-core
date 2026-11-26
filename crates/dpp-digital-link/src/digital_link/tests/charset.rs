//! The character set a qualifier may use.

use super::super::*;

/// AI 10 and AI 21 are `X..20`, so a decoded value must stay inside CSET 82.
/// Percent-encoding carries a character through the URI; it does not make it
/// one GS1 admits.
#[test]
fn a_qualifier_outside_cset_82_is_refused() {
    for (uri, code, character) in [
        (
            "https://id.example.com/01/09506000134352/21/SN%20001",
            "21",
            ' ',
        ),
        (
            "https://id.example.com/01/09506000134352/21/SN%23001",
            "21",
            '#',
        ),
        (
            "https://id.example.com/01/09506000134352/21/SN%C3%A9",
            "21",
            'é',
        ),
        (
            "https://id.example.com/01/09506000134352/10/LOT~A/21/X",
            "10",
            '~',
        ),
    ] {
        match DigitalLink::parse(uri) {
            Err(DigitalLinkError::OutsideCset82 {
                code: got_code,
                character: got,
            }) => assert_eq!((got_code.as_str(), got), (code, character), "{uri}"),
            other => panic!("{uri}: expected OutsideCset82, got {other:?}"),
        }
    }
}

/// The twenty CSET 82 symbols all survive a serial, percent-encoded where a
/// URI path needs them to be.
#[test]
fn every_cset_82_symbol_is_admitted_in_a_serial() {
    let symbols = "!\"%&'()*+,-./:;<=>?_";
    let uri = format!(
        "https://id.example.com/01/09506000134352/21/{}",
        codec::percent_encode(symbols)
    );
    let parsed = DigitalLink::parse(&uri).expect("every CSET 82 symbol is admitted");
    assert_eq!(parsed.serial(), Some(symbols));
}
