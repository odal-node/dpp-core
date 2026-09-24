//! The GS1 Digital Link a passport's data carrier encodes.

use dpp_domain::{Passport, ProductGroupData, ProductIdentifier};

use super::codec::percent_encode;
use super::error::DigitalLinkError;
use super::link::check_value;
use super::syntax_dictionary::ai_spec;

/// Build the GS1 Digital Link a passport's data carrier encodes, at the level
/// the passport describes:
///
/// | Level | Carrier |
/// |---|---|
/// | model | `{resolver_base}/01/{gtin}` |
/// | batch | `{resolver_base}/01/{gtin}/10/{batch}` |
/// | item, or not stated | `{resolver_base}/01/{gtin}/21/{serial}` |
///
/// What follows the GTIN is [`Passport::carrier_qualifier`], and that same value
/// is what a printed label is resolved by, so the carrier and the lookup read
/// one function and cannot drift.
///
/// # Why the level decides it
///
/// A GTIN with AI 21 is a serialised GTIN, which GS1 defines as identifying one
/// individual item. A model- or batch-level passport's carrier is printed on
/// every unit it covers, so a serial there would give all of them one
/// "individual" identity. The serial, where one is printed, is
/// [`Passport::effective_carrier_serial`] — the one the operator attributed, or
/// the default derived from the passport id — and never the manufacturer's
/// `serial_number`; an operator who wants that on the label attributes it.
///
/// A batch is printed only at batch level, where it is what the carrier
/// identifies. At item level the serial alone resolves the label, and the lot
/// is left off.
///
/// `Ok(None)` when the passport carries no EN 18219 scheme 1 identifier: a
/// Digital Link keyed on AI 01 needs a GTIN, and schemes 2 and 3 carry their own
/// URL or DID instead.
///
/// # Errors
///
/// [`DigitalLinkError::EmptyValue`], [`DigitalLinkError::ValueTooLong`] or
/// [`DigitalLinkError::OutsideCset82`] when the value to print — an attributed
/// serial in AI 21, or a batch-level passport's batch in AI 10 — is one GS1
/// would reject. A batch-level passport with no batch at all is
/// `EmptyValue("10")`. `Passport::validate` refuses the same records; this
/// holds for a passport that never went through it. A carrier is printed on
/// physical products, so it is refused here rather than emitted and found
/// wanting by a scanner.
///
/// [`DigitalLinkError::UnknownApplicationIdentifier`] if the vendored GS1
/// syntax dictionary has no entry for the AI being printed. That is an
/// invariant of a file compiled in with `include_str!`, not a condition a
/// caller can reach — but library code here does not panic on its own
/// invariants, so it is returned rather than asserted.
pub fn build_qr_url(
    resolver_base: &str,
    passport: &Passport,
) -> Result<Option<String>, DigitalLinkError> {
    let Some(gtin) = passport
        .product_group_data
        .as_ref()
        .and_then(ProductGroupData::product_identifier)
        .and_then(ProductIdentifier::gtin)
    else {
        return Ok(None);
    };

    let qualifier = passport
        .carrier_qualifier()
        .ok_or_else(|| DigitalLinkError::EmptyValue("10".to_owned()))?;
    let base = resolver_base.trim_end_matches('/');

    let Some((ai, value)) = qualifier.ai_element() else {
        return Ok(Some(format!("{base}/01/{}", gtin.as_str())));
    };
    if value.is_empty() {
        return Err(DigitalLinkError::EmptyValue(ai.to_owned()));
    }
    let spec =
        ai_spec(ai).ok_or_else(|| DigitalLinkError::UnknownApplicationIdentifier(ai.to_owned()))?;
    check_value(ai, spec, value)?;

    Ok(Some(format!(
        "{base}/01/{}/{ai}/{}",
        gtin.as_str(),
        percent_encode(value)
    )))
}
