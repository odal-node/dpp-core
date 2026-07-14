//! [`DigitalLink`] — a parsed GS1 Digital Link URI.

use dpp_domain::Gtin;

use super::ai::{AiRole, ai_descriptor};
use super::codec::{normalize_gtin_to_14, percent_decode, percent_encode};
use super::error::DigitalLinkError;

#[derive(Debug, Clone, PartialEq)]
pub struct DigitalLink {
    /// Base resolver URL including any path prefix before the `/01/` segment
    /// (e.g. `https://id.odal-node.io` or `https://example.com/resolve`).
    pub resolver_base: String,
    /// Validated 14-digit GTIN.
    pub gtin: Gtin,
    /// Consumer product variant (AI 22).
    pub variant: Option<String>,
    /// Batch / lot number (AI 10).
    pub batch: Option<String>,
    /// Serial number (AI 21).
    pub serial: Option<String>,
    /// Third-party controlled serial number (AI 235).
    pub tpcsn: Option<String>,
}

impl DigitalLink {
    /// Parse a GS1 Digital Link URI.
    ///
    /// Accepted forms:
    /// - `https://id.odal-node.io/01/09506000134352/21/ABC123`
    /// - `https://id.odal-node.io/01/09506000134352/10/BATCH01/21/SN001`
    /// - `https://example.com/resolve/01/09506000134352/21/SN001` (path prefix)
    ///
    /// GTIN-8 / GTIN-12 / GTIN-13 are normalised to 14 digits by left-padding.
    /// Unknown AI codes produce `UnknownApplicationIdentifier`.
    /// Qualifiers out of canonical order produce `QualifiersOutOfOrder`.
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

        // Locate the primary key (AI 01) — everything before it is the resolver
        // path prefix.
        let gtin_seg_pos = all_segments
            .iter()
            .position(|s| *s == "01")
            .ok_or(DigitalLinkError::MissingGtin)?;

        let path_prefix = if gtin_seg_pos > 0 {
            format!("/{}", all_segments[..gtin_seg_pos].join("/"))
        } else {
            String::new()
        };

        // Process AI segments starting at "01".
        let ai_segments = &all_segments[gtin_seg_pos..];
        let mut i = 0;
        let mut gtin: Option<Gtin> = None;
        let mut variant: Option<String> = None;
        let mut batch: Option<String> = None;
        let mut serial: Option<String> = None;
        let mut tpcsn: Option<String> = None;
        let mut last_qualifier_order: u8 = 0;
        let mut last_qualifier_code: &str = "";

        while i + 1 < ai_segments.len() {
            let code = ai_segments[i];
            let desc = ai_descriptor(code)
                .ok_or_else(|| DigitalLinkError::UnknownApplicationIdentifier(code.to_owned()))?;

            let raw_value = ai_segments[i + 1];
            let value = percent_decode(raw_value);

            // GS1 mandates a maximum length per AI; enforce it so an untrusted
            // URI cannot smuggle an unbounded value downstream.
            let value_len = value.chars().count();
            if value_len > desc.max_len {
                return Err(DigitalLinkError::ValueTooLong {
                    code: code.to_owned(),
                    max_len: desc.max_len,
                    actual: value_len,
                });
            }

            match desc.role {
                AiRole::PrimaryKey => {
                    // A second '01' segment must not silently overwrite the
                    // GTIN parsed from the first.
                    if gtin.is_some() {
                        return Err(DigitalLinkError::DuplicatePrimaryKey);
                    }
                    let padded = normalize_gtin_to_14(&value)?;
                    gtin = Some(Gtin::parse(&padded)?);
                }
                AiRole::Qualifier => {
                    let order = desc.qualifier_order.unwrap_or(0);
                    if order <= last_qualifier_order && last_qualifier_order > 0 {
                        return Err(DigitalLinkError::QualifiersOutOfOrder {
                            before: last_qualifier_code.to_owned(),
                            before_ord: last_qualifier_order,
                            after: code.to_owned(),
                            after_ord: order,
                        });
                    }
                    last_qualifier_order = order;
                    last_qualifier_code = code;
                    match code {
                        "22" => variant = Some(value),
                        "10" => batch = Some(value),
                        "21" => serial = Some(value),
                        "235" => tpcsn = Some(value),
                        _ => {}
                    }
                }
                AiRole::DataAttribute => {
                    // Informational only; silently accepted.
                }
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

        let gtin = gtin.ok_or(DigitalLinkError::MissingGtin)?;

        Ok(Self {
            resolver_base: format!("https://{host}{path_prefix}"),
            gtin,
            variant,
            batch,
            serial,
            tpcsn,
        })
    }

    /// Build a canonical GS1 Digital Link URI with qualifiers in standard order.
    ///
    /// AI values containing reserved characters are percent-encoded.
    pub fn build(&self) -> String {
        let mut uri = format!(
            "{}/01/{}",
            self.resolver_base.trim_end_matches('/'),
            self.gtin.as_str()
        );
        if let Some(v) = &self.variant {
            uri.push_str(&format!("/22/{}", percent_encode(v)));
        }
        if let Some(b) = &self.batch {
            uri.push_str(&format!("/10/{}", percent_encode(b)));
        }
        if let Some(s) = &self.serial {
            uri.push_str(&format!("/21/{}", percent_encode(s)));
        }
        if let Some(t) = &self.tpcsn {
            uri.push_str(&format!("/235/{}", percent_encode(t)));
        }
        uri
    }
}
