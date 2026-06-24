//! ISO 3166-1 alpha-2 country code validation.

/// Officially assigned ISO 3166-1 alpha-2 codes (sorted for binary search).
///
/// Excludes exceptionally/transitionally reserved codes (e.g. `EU`, `UK`): only
/// codes ISO 3166-1 has officially assigned to a country are accepted. Kept as a
/// `const` so this stays `no_std` + zero-dependency.
const ISO_3166_1_ALPHA2: &[&str] = &[
    "AD", "AE", "AF", "AG", "AI", "AL", "AM", "AO", "AQ", "AR", "AS", "AT", "AU", "AW", "AX", "AZ",
    "BA", "BB", "BD", "BE", "BF", "BG", "BH", "BI", "BJ", "BL", "BM", "BN", "BO", "BQ", "BR", "BS",
    "BT", "BV", "BW", "BY", "BZ", "CA", "CC", "CD", "CF", "CG", "CH", "CI", "CK", "CL", "CM", "CN",
    "CO", "CR", "CU", "CV", "CW", "CX", "CY", "CZ", "DE", "DJ", "DK", "DM", "DO", "DZ", "EC", "EE",
    "EG", "EH", "ER", "ES", "ET", "FI", "FJ", "FK", "FM", "FO", "FR", "GA", "GB", "GD", "GE", "GF",
    "GG", "GH", "GI", "GL", "GM", "GN", "GP", "GQ", "GR", "GS", "GT", "GU", "GW", "GY", "HK", "HM",
    "HN", "HR", "HT", "HU", "ID", "IE", "IL", "IM", "IN", "IO", "IQ", "IR", "IS", "IT", "JE", "JM",
    "JO", "JP", "KE", "KG", "KH", "KI", "KM", "KN", "KP", "KR", "KW", "KY", "KZ", "LA", "LB", "LC",
    "LI", "LK", "LR", "LS", "LT", "LU", "LV", "LY", "MA", "MC", "MD", "ME", "MF", "MG", "MH", "MK",
    "ML", "MM", "MN", "MO", "MP", "MQ", "MR", "MS", "MT", "MU", "MV", "MW", "MX", "MY", "MZ", "NA",
    "NC", "NE", "NF", "NG", "NI", "NL", "NO", "NP", "NR", "NU", "NZ", "OM", "PA", "PE", "PF", "PG",
    "PH", "PK", "PL", "PM", "PN", "PR", "PS", "PT", "PW", "PY", "QA", "RE", "RO", "RS", "RU", "RW",
    "SA", "SB", "SC", "SD", "SE", "SG", "SH", "SI", "SJ", "SK", "SL", "SM", "SN", "SO", "SR", "SS",
    "ST", "SV", "SX", "SY", "SZ", "TC", "TD", "TF", "TG", "TH", "TJ", "TK", "TL", "TM", "TN", "TO",
    "TR", "TT", "TV", "TW", "TZ", "UA", "UG", "UM", "US", "UY", "UZ", "VA", "VC", "VE", "VG", "VI",
    "VN", "VU", "WF", "WS", "YE", "YT", "ZA", "ZM", "ZW",
];

/// ISO 3166-1 alpha-2: two uppercase ASCII letters that are an officially
/// assigned country code. Shape-only candidates such as `XX`/`QZ`/`AA` are
/// rejected because they are not in the assigned set.
#[must_use]
pub fn country_code_valid(code: &str) -> bool {
    code.len() == 2
        && code.bytes().all(|b| b.is_ascii_uppercase())
        && ISO_3166_1_ALPHA2.binary_search(&code).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn country_code_rules() {
        assert!(country_code_valid("DE"));
        assert!(!country_code_valid("de"));
        assert!(!country_code_valid("DEU"));
    }

    #[test]
    fn assigned_codes_accepted() {
        for c in ["DE", "FR", "US", "MK", "SS", "AD", "ZW"] {
            assert!(country_code_valid(c), "{c} is an assigned ISO 3166-1 code");
        }
    }

    #[test]
    fn well_formed_but_unassigned_codes_rejected() {
        // Correct shape, not officially assigned ⇒ must be rejected.
        for c in ["XX", "QZ", "AA", "ZZ", "OO"] {
            assert!(!country_code_valid(c), "{c} is not an assigned code");
        }
        // `EU`/`UK` are reserved, not officially assigned.
        assert!(!country_code_valid("EU"));
        assert!(!country_code_valid("UK"));
    }

    #[test]
    fn table_is_sorted_for_binary_search() {
        assert!(ISO_3166_1_ALPHA2.windows(2).all(|w| w[0] < w[1]));
    }
}
