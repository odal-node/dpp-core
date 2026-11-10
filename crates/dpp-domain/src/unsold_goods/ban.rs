//! [`destruction_ban_applies`] — when ESPR Art. 25(1) bars a destruction.
//!
//! ✅ COMPLIANCE-PIN: Regulation (EU) 2024/1781 Art. 25(1) (consolidated 28.6.2024):
//! the ban starts on 19 July 2026 and does not apply to micro and small enterprises,
//! nor to medium-sized ones before 19 July 2030. Read against the Official Journal
//! text.

use chrono::NaiveDate;
use dpp_rules::unsold_goods::annex_vii::is_within_annex_vii_scope;

use super::exemption::EnterpriseSize;

/// The day the destruction of the unsold products of Annex VII became prohibited
/// for the enterprises the ban reaches: 19 July 2026.
pub const DESTRUCTION_BAN_FROM: NaiveDate = match NaiveDate::from_ymd_opt(2026, 7, 19) {
    Some(date) => date,
    None => panic!("19 July 2026 is a date"),
};

/// Whether ESPR Art. 25(1) bars destroying a product on `on`, so that the
/// destruction needs a derogation (Del. Reg. (EU) 2026/296 Art. 2) to be lawful.
///
/// It does when **all** of these hold:
///
/// - the product is an unsold consumer product, which a passport does not say and
///   the caller therefore does: a product destroyed after use is outside Art. 25;
/// - a CN code of the product is in Annex VII (apparel, clothing accessories or
///   footwear), by prefix, as `cn_codes` are 6, 8 or 10 digits;
/// - the day is on or after [`DESTRUCTION_BAN_FROM`];
/// - the operator is not exempt on that day ([`EnterpriseSize::is_exempt_on`]).
///
/// Anything else may be destroyed without citing a derogation. A code that is not
/// readable is not in Annex VII, so a caller with a malformed one is told the ban
/// does not apply: this answers what the data say, and reporting the malformed code
/// is the lints' work.
#[must_use]
pub fn destruction_ban_applies(
    unsold: bool,
    cn_codes: &[&str],
    size: EnterpriseSize,
    on: NaiveDate,
) -> bool {
    unsold
        && on >= DESTRUCTION_BAN_FROM
        && !size.is_exempt_on(on)
        && cn_codes.iter().any(|code| is_within_annex_vii_scope(code))
}
