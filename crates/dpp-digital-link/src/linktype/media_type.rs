//! Media types relevant to DPP content negotiation.

use serde::{Deserialize, Serialize};

/// Media types relevant to DPP content negotiation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DppMediaType {
    /// `application/json` — raw DPP JSON data.
    Json,
    /// `application/ld+json` — JSON-LD representation with context.
    JsonLd,
    /// `text/html` — human-readable product information page.
    Html,
    /// `application/pdf` — PDF version of the DPP.
    Pdf,
    /// `application/cbor` — compact binary representation.
    Cbor,
    /// Custom media type.
    Custom(String),
}

impl DppMediaType {
    /// Parse from a MIME type string.
    pub fn parse(s: &str) -> Self {
        // Strip parameters (e.g., "; charset=utf-8")
        let base = s.split(';').next().unwrap_or(s).trim();
        match base {
            "application/json" => Self::Json,
            "application/ld+json" => Self::JsonLd,
            "text/html" => Self::Html,
            "application/pdf" => Self::Pdf,
            "application/cbor" => Self::Cbor,
            other => Self::Custom(other.to_owned()),
        }
    }

    /// Return the MIME type string.
    pub fn as_mime(&self) -> &str {
        match self {
            Self::Json => "application/json",
            Self::JsonLd => "application/ld+json",
            Self::Html => "text/html",
            Self::Pdf => "application/pdf",
            Self::Cbor => "application/cbor",
            Self::Custom(s) => s.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_as_mime_round_trip_for_known_types() {
        let cases = [
            (DppMediaType::Json, "application/json"),
            (DppMediaType::JsonLd, "application/ld+json"),
            (DppMediaType::Html, "text/html"),
            (DppMediaType::Pdf, "application/pdf"),
            (DppMediaType::Cbor, "application/cbor"),
        ];
        for (variant, mime) in cases {
            assert_eq!(DppMediaType::parse(mime), variant);
            assert_eq!(variant.as_mime(), mime);
        }
    }

    #[test]
    fn parse_strips_parameters() {
        assert_eq!(
            DppMediaType::parse("application/json; charset=utf-8"),
            DppMediaType::Json
        );
        // Leading/trailing whitespace around the base type is trimmed.
        assert_eq!(DppMediaType::parse("  text/html "), DppMediaType::Html);
    }

    #[test]
    fn unknown_mime_becomes_custom() {
        let custom = DppMediaType::parse("application/x-protobuf");
        assert_eq!(
            custom,
            DppMediaType::Custom("application/x-protobuf".to_owned())
        );
        assert_eq!(custom.as_mime(), "application/x-protobuf");
    }
}
