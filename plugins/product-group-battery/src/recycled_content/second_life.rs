//! Art. 8(4)'s second-life carve-out, read off the payload's `batteryStatus`.

use dpp_plugin_sdk::rules::batteries::recycled_content::Art8SecondLife;
use dpp_plugin_sdk::traits::PluginInput;
use serde_json::Value;

/// Read Art. 8(4)'s second-life test off the payload's `batteryStatus`.
///
/// `batteryStatus` is Annex XIII point 4(c), whose closed set is
/// `original | repurposed | re-used | remanufactured | waste`. Three of those
/// name an Art. 8(4) operation. `waste` does not: end-of-life is not preparation
/// for re-use, and a waste battery is not being placed on the market.
///
/// A battery reaching one of those three states was necessarily on the market
/// before it — you cannot repurpose what was never placed — so the article's
/// second condition is satisfied by the status itself. That inference is the
/// operator's to stand behind; the field is theirs to declare.
///
/// **Absent or unrecognised reads as `None`.** Art. 8(4) is an exemption, and an
/// undeclared status is not evidence of one. Excusing a duty on a missing field
/// is the error an authority finds later; asserting one we should not is the
/// error the operator can see and correct now.
pub(super) fn art8_second_life(input: &PluginInput) -> Art8SecondLife {
    match input.get("batteryStatus").and_then(Value::as_str) {
        Some("repurposed" | "re-used" | "remanufactured") => Art8SecondLife::PlacedBeforeOperations,
        _ => Art8SecondLife::None,
    }
}
