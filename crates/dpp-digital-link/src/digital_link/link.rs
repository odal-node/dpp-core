//! [`DigitalLink`] — a parsed GS1 Digital Link URI.

use std::borrow::Cow;

use dpp_domain::{CarrierQualifier, Gtin};

use super::codec::{
    is_segment, is_valid_authority, normalize_gtin_to_14, percent_decode, percent_encode,
};
use super::error::DigitalLinkError;
use super::primary_key::PrimaryKey;
use super::syntax_dictionary::{ai_spec, qualifier_position, required_qualifier};
use super::value::check_value;

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
    /// - `https://id.odal-node.io/00/106141411234567897` (any GS1 primary key)
    ///
    /// GTIN-8 / GTIN-12 / GTIN-13 are normalised to 14 digits by left-padding.
    /// Unknown AI codes produce `UnknownApplicationIdentifier`; qualifiers out
    /// of canonical order produce `QualifiersOutOfOrder`; a path with no
    /// primary key at all produces `MissingGtin`.
    pub fn parse(uri: &str) -> Result<Self, DigitalLinkError> {
        // Strip query string so `?linkType=…` never corrupts the last value.
        let path_end = uri.find('?').unwrap_or(uri.len());
        let uri_no_query = &uri[..path_end];

        // The grammar's `scheme` is one of four spellings: `http` and `https`, and
        // the same two in capitals. It is kept as written, so a link read from a
        // label builds back to the same base.
        let (scheme, without_scheme) = uri_no_query
            .split_once("://")
            .filter(|(scheme, _)| matches!(*scheme, "http" | "https" | "HTTP" | "HTTPS"))
            .ok_or_else(|| {
                DigitalLinkError::InvalidScheme(
                    uri_no_query.split("://").next().unwrap_or("").to_owned(),
                )
            })?;
        let slash_pos = without_scheme.find('/').unwrap_or(without_scheme.len());
        let host = &without_scheme[..slash_pos];
        let path = &without_scheme[slash_pos..];
        if !is_valid_authority(host) {
            return Err(DigitalLinkError::InvalidHost(host.to_owned()));
        }

        // `path` is empty or starts with `/`. One trailing slash is tolerated, as
        // resolvers are asked to; every other empty segment is kept, so the
        // checks below see it.
        let body = path.strip_suffix('/').unwrap_or(path);
        let all_segments: Vec<&str> = body.split('/').skip(1).collect();

        // Locate the primary key — everything before it is the resolver path
        // prefix. Read from the dictionary's `dlpkey` flag, so the set tracks
        // GS1's designations rather than a snapshot of them.
        let key_pos = all_segments
            .iter()
            .position(|s| ai_spec(s).is_some_and(|spec| spec.dl_primary_key))
            .ok_or(DigitalLinkError::MissingGtin)?;

        // The stem's segments are the grammar's `segment`, which may be empty.
        // From the primary key on, each segment is an AI or a value: never empty,
        // and in a value the double quote, which `XSYMBOL` names as itself, is the
        // one character beyond `pchar` the grammar writes raw. A `%` is let through
        // here so that `percent_decode` names a malformed escape as one.
        let (stem, ai_segments) = all_segments.split_at(key_pos);
        let bad_stem = stem.iter().find(|s| !is_segment(s, b""));
        let bad_ai = ai_segments
            .iter()
            .find(|s| s.is_empty() || !is_segment(s, b"\"%"));
        if let Some(bad) = bad_stem.or(bad_ai) {
            return Err(DigitalLinkError::InvalidPathSegment((*bad).to_owned()));
        }

        let stem: Vec<&str> = stem.iter().copied().filter(|s| !s.is_empty()).collect();
        let path_prefix = if stem.is_empty() {
            String::new()
        } else {
            format!("/{}", stem.join("/"))
        };

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
            let value = percent_decode(raw_value)?;
            // A GTIN of fewer than 14 digits is the legacy form, which the
            // grammar no longer has and which this reader still pads. It is
            // checked by `Gtin::parse` below, in its own words, rather than
            // here; every other AI is held to its dictionary entry.
            let value = if code == "01" {
                normalize_gtin_to_14(&value)?
            } else {
                check_value(code, spec, &value)?;
                value
            };

            if spec.dl_primary_key {
                // A second primary key must not silently overwrite the first —
                // whether it repeats the same AI or names another of GS1's.
                if primary_key.is_some() {
                    return Err(DigitalLinkError::DuplicatePrimaryKey);
                }
                primary_ai = code;
                primary_key = Some(if code == "01" {
                    // The one key this workspace models as a validated type.
                    PrimaryKey::Gtin(Gtin::parse(&value)?)
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

        // A key that GS1's dictionary makes dependent on one of its qualifiers
        // is not a link without it: AI 415 is a payer, and is named only
        // together with the payment reference it was invoiced under.
        if let Some(required) = required_qualifier(primary_ai)
            && !qualifiers.iter().any(|(ai, _)| ai == required)
        {
            return Err(DigitalLinkError::MissingQualifier {
                primary_key: primary_ai.to_owned(),
                qualifier: required.to_owned(),
            });
        }

        Ok(Self {
            resolver_base: format!("{scheme}://{host}{path_prefix}"),
            primary_key,
            qualifiers,
        })
    }

    /// Build a GS1 Digital Link URI with qualifiers in path order.
    ///
    /// Every AI value, the primary key's included, is written with each
    /// character outside the unreserved set percent-encoded, which is how the
    /// grammar spells them. A CPID such as `AB-C#/` therefore leaves as
    /// `AB-C%23%2F` rather than as a fragment and a new path segment.
    pub fn build(&self) -> String {
        let mut uri = format!(
            "{}/{}/{}",
            self.resolver_base.trim_end_matches('/'),
            self.primary_key.ai(),
            percent_encode(self.primary_key.wire_value())
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

    /// The passport-carrier qualifier this link names, to resolve it by with
    /// `PassportRepository::find_by_carrier` — the reading of
    /// [`Passport::carrier_qualifier`](dpp_domain::Passport::carrier_qualifier)
    /// that [`build_qr_url`](crate::build_qr_url) printed.
    ///
    /// | Path after the GTIN | Qualifier |
    /// |---|---|
    /// | nothing | `Model` |
    /// | `/10/{lot}` | `Batch(lot)` |
    /// | `/21/{sn}` | `Serial(sn)` |
    /// | `/10/{lot}/21/{sn}` | `Serial(sn)` |
    ///
    /// A serial wins over a lot because the GTIN and the serial already name
    /// one unit, and because carriers printed by earlier versions of this crate
    /// carried both — the lot there qualifies nothing the serial does not.
    ///
    /// `None` for a link no passport carrier is: one keyed on anything but a
    /// GTIN, or one carrying AI 22 or AI 235, which no carrier here prints.
    /// Dropping an unknown qualifier to find *something* would answer for a
    /// different thing than the label names.
    #[must_use]
    pub fn carrier_qualifier(&self) -> Option<CarrierQualifier<'_>> {
        self.gtin()?;
        if self.variant().is_some() || self.tpcsn().is_some() {
            return None;
        }
        Some(match (self.batch(), self.serial()) {
            (_, Some(serial)) => CarrierQualifier::Serial(Cow::Borrowed(serial)),
            (Some(batch), None) => CarrierQualifier::Batch(Cow::Borrowed(batch)),
            (None, None) => CarrierQualifier::Model,
        })
    }
}
