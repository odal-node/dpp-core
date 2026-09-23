//! The GS1 Digital Link a passport's data carrier encodes.

use dpp_domain::{Passport, ProductGroupData, ProductIdentifier};

use super::codec::percent_encode;
use super::error::DigitalLinkError;
use super::link::check_value;
use super::syntax_dictionary::ai_spec;

/// Build the GS1 Digital Link a passport's data carrier encodes:
/// `{resolver_base}/01/{gtin}/21/{serial}`.
///
/// The serial is [`Passport::effective_carrier_serial`] — the one the operator
/// attributed, or the default derived from the passport id — and that same
/// value is what a printed label is resolved by, so the two read one field and
/// cannot drift. It is never the manufacturer's `serial_number`; an operator
/// who wants that on the label attributes it.
///
/// `Ok(None)` when the passport carries no EN 18219 scheme 1 identifier: a
/// Digital Link keyed on AI 01 needs a GTIN, and schemes 2 and 3 carry their own
/// URL or DID instead.
///
/// # No batch
///
/// AI 10 is not emitted. The serial alone resolves the label to its passport,
/// so a lot adds nothing to resolution — and a lot is operator free text, which
/// would put a value this crate cannot vouch for into every printed code.
///
/// # Errors
///
/// [`DigitalLinkError::EmptyValue`], [`DigitalLinkError::ValueTooLong`] or
/// [`DigitalLinkError::OutsideCset82`] when an attributed serial is one GS1
/// would reject in AI 21. `Passport::validate` refuses the same values; this
/// holds for a passport that never went through it. A carrier is
/// printed on physical products, so it is refused here rather than emitted and
/// found wanting by a scanner.
///
/// [`DigitalLinkError::UnknownApplicationIdentifier`] if the vendored GS1
/// syntax dictionary has no AI 21. That is an invariant of a file compiled in
/// with `include_str!`, not a condition a caller can reach — but library code
/// here does not panic on its own invariants, so it is returned rather than
/// asserted.
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

    let serial = passport.effective_carrier_serial();
    if serial.is_empty() {
        return Err(DigitalLinkError::EmptyValue("21".to_owned()));
    }
    let spec = ai_spec("21")
        .ok_or_else(|| DigitalLinkError::UnknownApplicationIdentifier("21".to_owned()))?;
    check_value("21", spec, &serial)?;

    Ok(Some(format!(
        "{}/01/{}/21/{}",
        resolver_base.trim_end_matches('/'),
        gtin.as_str(),
        percent_encode(&serial)
    )))
}
