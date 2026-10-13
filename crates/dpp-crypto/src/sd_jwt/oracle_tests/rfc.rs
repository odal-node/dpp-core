//! RFC 9901's own SD-JWTs, exactly as vendored in this repository.
//!
//! The strings come from `rfc9901_examples.rs`, not from the RFC, and that is the
//! point. Those strings were copied by hand with the line breaks of the text
//! removed. A transcription slip in one would pass every test that only parses it
//! with the code that wrote the tests. An independent implementation verifying
//! the RFC's signature over a token catches it: the signature covers every byte
//! of the Issuer-signed JWT, and the Key Binding JWT's `sd_hash` covers the whole
//! presentation.

use serde_json::Value;

use crate::sd_jwt::rfc9901_examples::{
    A1_PRESENTATION, A1_PROCESSED, A2_PRESENTATION, A2_PROCESSED, A3_ISSUED, A3_PRESENTATION,
    A3_PROCESSED, A4_PRESENTATION, A4_PROCESSED, S51_ISSUED, S52_PRESENTATION, S52_PROCESSED,
};

pub(super) struct RfcCase {
    pub(super) id: &'static str,
    pub(super) token: &'static str,
    /// The Processed SD-JWT Payload the RFC prints for it, where it prints one.
    pub(super) printed: Option<Value>,
}

fn json_of(text: &str) -> Value {
    serde_json::from_str(text).expect("the RFC's JSON parses")
}

pub(super) fn cases() -> Vec<RfcCase> {
    let with = |id, token, printed: &str| RfcCase {
        id,
        token,
        printed: Some(json_of(printed)),
    };
    vec![
        with("appendix-a1-presentation", A1_PRESENTATION, A1_PROCESSED),
        with("appendix-a2-presentation", A2_PRESENTATION, A2_PROCESSED),
        with(
            "appendix-a3-presentation-with-key-binding",
            A3_PRESENTATION,
            A3_PROCESSED,
        ),
        with(
            "appendix-a4-presentation-with-key-binding",
            A4_PRESENTATION,
            A4_PROCESSED,
        ),
        with("section-5-2-presentation", S52_PRESENTATION, S52_PROCESSED),
        // The RFC prints these two as issued but not as processed.
        RfcCase {
            id: "appendix-a3-issued",
            token: A3_ISSUED,
            printed: None,
        },
        RfcCase {
            id: "section-5-1-issued",
            token: S51_ISSUED,
            printed: None,
        },
    ]
}
