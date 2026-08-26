//! [`DigitalLink`] — a parsed GS1 Digital Link URI.

use dpp_domain::Gtin;

use super::codec::{normalize_gtin_to_14, percent_decode, percent_encode};
use super::error::DigitalLinkError;
use super::primary_key::PrimaryKey;
use super::syntax_dictionary::{ai_spec, qualifier_position};

/// A parsed GS1 Digital Link URI.
///
/// The path is a **primary key** followed by that key's own qualifiers. Which
/// AIs may open a path, and which qualifiers each accepts in what order, comes
/// from GS1's Barcode Syntax Dictionary rather than from anything encoded here
/// — see [`crate::ai_spec`] and [`crate::qualifier_position`].
#[derive(Debug, Clone, PartialEq)]
pub struct DigitalLink {
    /// Base resolver URL including any path prefix before the primary-key
    /// segment (e.g. `https://id.odal-node.io` or `https://example.com/resolve`).
    pub resolver_base: String,
    /// The AI the path opens on.
    pub primary_key: PrimaryKey,
    /// The qualifiers that followed it, as `(ai, value)`, in path order.
    ///
    /// A list rather than named fields because the qualifier set is a property
    /// of the primary key: AI `01` accepts `22,10,21` or `235`, AI `414`
    /// accepts `254` or `7040`, AI `8010` accepts `8011`. Naming one key's
    /// qualifiers as struct fields would have made this type GTIN-shaped
    /// forever, which is how it came to refuse the other fifteen keys.
    ///
    /// [`DigitalLink::qualifier`] and the GTIN-specific accessors below cover
    /// the common lookups.
    pub qualifiers: Vec<(String, String)>,
}

impl DigitalLink {
    /// Parse a GS1 Digital Link URI.
    ///
    /// Accepted forms:
    /// - `https://id.odal-node.io/01/09506000134352/21/ABC123`
    /// - `https://id.odal-node.io/01/09506000134352/10/BATCH01/21/SN001`
    /// - `https://example.com/resolve/01/09506000134352/21/SN001` (path prefix)
    /// - `https://id.odal-node.io/00/106141411234567890` (any GS1 primary key)
    ///
    /// GTIN-8 / GTIN-12 / GTIN-13 are normalised to 14 digits by left-padding.
    /// Unknown AI codes produce `UnknownApplicationIdentifier`; qualifiers out
    /// of canonical order produce `QualifiersOutOfOrder`; a path with no
    /// primary key at all produces `MissingGtin`.
    pub fn parse(uri: &str) -> Result<Self, DigitalLinkError> {
        // Strip query string so `?linkType=…` never corrupts the last value.
        let path_end = uri.find('?').unwrap_or(uri.len());
        let uri_no_query = &uri[..path_end];

        if !uri_no_query.starts_with("https://") {
            let scheme = uri_no_query.split("://").next().unwrap_or("").to_owned();
            return Err(DigitalLinkError::InvalidScheme(scheme));
        }

        let without_scheme = &uri_no_query["https://".len()..];
        let slash_pos = without_scheme.find('/').unwrap_or(without_scheme.len());
        let host = &without_scheme[..slash_pos];
        let path = &without_scheme[slash_pos..];

        let all_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        // Locate the primary key — everything before it is the resolver path
        // prefix. Read from the dictionary's `dlpkey` flag, so the set tracks
        // GS1's designations rather than a snapshot of them.
        let key_pos = all_segments
            .iter()
            .position(|s| ai_spec(s).is_some_and(|spec| spec.dl_primary_key))
            .ok_or(DigitalLinkError::MissingGtin)?;

        let path_prefix = if key_pos > 0 {
            format!("/{}", all_segments[..key_pos].join("/"))
        } else {
            String::new()
        };

        let ai_segments = &all_segments[key_pos..];
        let mut i = 0;
        let mut primary_key: Option<PrimaryKey> = None;
        let mut primary_ai = "";
        let mut qualifiers: Vec<(String, String)> = Vec::new();
        // The qualifier last seen: its alternative-sequence index and its AI.
        let mut last_qualifier: Option<(usize, &str)> = None;

        while i + 1 < ai_segments.len() {
            let code = ai_segments[i];
            let spec = ai_spec(code)
                .ok_or_else(|| DigitalLinkError::UnknownApplicationIdentifier(code.to_owned()))?;

            let raw_value = ai_segments[i + 1];
            let value = percent_decode(raw_value);

            // GS1 mandates a maximum length per AI; enforce it so an untrusted
            // URI cannot smuggle an unbounded value downstream.
            let value_len = value.chars().count();
            if value_len > spec.max_len {
                return Err(DigitalLinkError::ValueTooLong {
                    code: code.to_owned(),
                    max_len: spec.max_len,
                    actual: value_len,
                });
            }

            if spec.dl_primary_key {
                // A second primary key must not silently overwrite the first —
                // whether it repeats the same AI or names another of GS1's.
                if primary_key.is_some() {
                    return Err(DigitalLinkError::DuplicatePrimaryKey);
                }
                primary_ai = code;
                primary_key = Some(if code == "01" {
                    // The one key this workspace models as a validated type.
                    PrimaryKey::Gtin(Gtin::parse(&normalize_gtin_to_14(&value)?)?)
                } else {
                    PrimaryKey::Other {
                        ai: code.to_owned(),
                        unvalidated_value: value,
                    }
                });
            } else if let Some((seq, order)) = qualifier_position(primary_ai, code) {
                // Alternatives first: two qualifiers from different sequences
                // describe no link GS1 defines, and the order check below would
                // otherwise compare positions that are not comparable.
                if let Some((last_seq, last_code)) = last_qualifier
                    && last_seq != seq
                {
                    return Err(DigitalLinkError::MixedQualifierSequences {
                        primary_key: primary_ai.to_owned(),
                        first: last_code.to_owned(),
                        second: code.to_owned(),
                    });
                }
                if let Some((_, last_code)) = last_qualifier
                    && let Some((_, last_ord)) = qualifier_position(primary_ai, last_code)
                    && order <= last_ord
                {
                    return Err(DigitalLinkError::QualifiersOutOfOrder {
                        before: last_code.to_owned(),
                        before_ord: last_ord,
                        after: code.to_owned(),
                        after_ord: order,
                    });
                }
                last_qualifier = Some((seq, code));
                qualifiers.push((code.to_owned(), value));
            } else {
                // Known to the dictionary, but neither this primary key nor one
                // of its qualifiers. GS1's Digital Link grammar puts only the
                // primary key and its qualifiers in the path; a data attribute
                // belongs in the query string. Confirmed against the GS1
                // Barcode Syntax Engine, which rejects `/21/SN001/99/…`.
                return Err(DigitalLinkError::DataAttributeInPath(code.to_owned()));
            }

            i += 2;
        }

        // An odd segment count leaves a trailing AI code with no value — reject
        // it rather than silently dropping the dangling qualifier.
        if i < ai_segments.len() {
            return Err(DigitalLinkError::TrailingUnpairedSegment(
                ai_segments[i].to_owned(),
            ));
        }

        let primary_key = primary_key.ok_or(DigitalLinkError::MissingGtin)?;

        Ok(Self {
            resolver_base: format!("https://{host}{path_prefix}"),
            primary_key,
            qualifiers,
        })
    }

    /// Build a canonical GS1 Digital Link URI with qualifiers in path order.
    ///
    /// AI values containing reserved characters are percent-encoded.
    pub fn build(&self) -> String {
        let mut uri = format!(
            "{}/{}/{}",
            self.resolver_base.trim_end_matches('/'),
            self.primary_key.ai(),
            self.primary_key.wire_value()
        );
        for (ai, value) in &self.qualifiers {
            uri.push_str(&format!("/{ai}/{}", percent_encode(value)));
        }
        uri
    }

    /// The value of one qualifier, if the path carried it.
    #[must_use]
    pub fn qualifier(&self, ai: &str) -> Option<&str> {
        self.qualifiers
            .iter()
            .find(|(code, _)| code == ai)
            .map(|(_, value)| value.as_str())
    }

    /// The validated GTIN, when the path is keyed on AI `01`.
    #[must_use]
    pub fn gtin(&self) -> Option<&Gtin> {
        self.primary_key.as_gtin()
    }

    /// Consumer product variant, AI `22` — a GTIN qualifier.
    #[must_use]
    pub fn variant(&self) -> Option<&str> {
        self.qualifier("22")
    }

    /// Batch / lot number, AI `10` — a GTIN qualifier.
    #[must_use]
    pub fn batch(&self) -> Option<&str> {
        self.qualifier("10")
    }

    /// Serial number, AI `21` — a GTIN qualifier.
    #[must_use]
    pub fn serial(&self) -> Option<&str> {
        self.qualifier("21")
    }

    /// Third-party controlled serial number, AI `235` — a GTIN qualifier.
    #[must_use]
    pub fn tpcsn(&self) -> Option<&str> {
        self.qualifier("235")
    }
}
