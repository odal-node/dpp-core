//! The payload fields that declare each metal's Art. 8 recycled-content share.

use dpp_plugin_sdk::rules::batteries::recycled_content::RecycledContentInput;
use dpp_plugin_sdk::traits::PluginInput;
use dpp_plugin_sdk::validate::num;

/// The field declaring `metal`'s share, for each metal Art. 8 sets a minimum
/// for.
///
/// The one place a metal is tied to its field. Reading the shares and pointing
/// a finding at one of them both go through here, so the two cannot drift
/// apart. They were written out twice before, in two `match`es.
fn share_field(metal: &str) -> Option<&'static str> {
    match metal {
        "cobalt" => Some("recycledContentCobaltPct"),
        "lithium" => Some("recycledContentLithiumPct"),
        "nickel" => Some("recycledContentNickelPct"),
        "lead" => Some("recycledContentLeadPct"),
        _ => None,
    }
}

/// The shares the payload declares. An undeclared metal is `None`.
pub(crate) fn declared_shares(input: &PluginInput) -> RecycledContentInput {
    let share = |metal: &str| share_field(metal).and_then(|field| num(input, field));
    RecycledContentInput {
        cobalt_pct: share("cobalt"),
        lithium_pct: share("lithium"),
        nickel_pct: share("nickel"),
        lead_pct: share("lead"),
    }
}

/// The JSON pointer a finding about `metal`'s share names: empty for a metal
/// Art. 8 does not cover.
pub(super) fn share_pointer(metal: &str) -> String {
    share_field(metal).map_or_else(String::new, |field| format!("/{field}"))
}
