//! The data of RFC 9901's own examples, copied from the RFC.
//!
//! Copied verbatim, because the point of using them is that this code did not
//! produce them: the base64url strings are the RFC's, with the line breaks of the
//! text removed, and the JSON is the RFC's, in its layout. The RFC is published
//! under the IETF Trust's Legal Provisions Relating to IETF Documents, and the
//! examples are reused here as Code Components under its terms:
//!
//! > Copyright (c) 2025 IETF Trust and the persons identified as authors of the
//! > code. All rights reserved.
//! >
//! > Redistribution and use in source and binary forms, with or without
//! > modification, is permitted pursuant to, and subject to the license terms
//! > contained in, the Revised BSD License set forth in Section 4.c of the IETF
//! > Trust's Legal Provisions Relating to IETF Documents
//! > (<https://trustee.ietf.org/license-info>).
//!
//! What is here, by where the RFC has it:
//!
//! - Appendix A.1, A.3 and A.4: complete SD-JWTs, issued or presented, with
//!   decoy digests, recursive Disclosures and a Key Binding JWT, and the
//!   Processed SD-JWT Payload the RFC prints for each.
//! - Sections 6.1 to 6.3: three ways of structuring one address claim, with each
//!   Disclosure and the digest the RFC prints for it.
//! - Sections 5.1 and 5.2 and Appendix A.2: the SD-JWTs that hide array elements,
//!   which clause 4.2.2 defines. Section 5 issues two nationalities as two
//!   Disclosures and presents one; A.2 discloses an array element whose value
//!   holds further digests.

/// Appendix A.1: a presentation that discloses only the region and the country of the address.
pub(super) const A1_PRESENTATION: &str = concat!(
    "eyJhbGciOiAiRVMyNTYiLCAidHlwIjogImV4YW1wbGUrc2Qtand0In0.",
    "eyJfc2Qi",
    "OiBbIkM5aW5wNllvUmFFWFI0Mjd6WUpQN1FyazFXSF84YmR3T0FfWVVyVW5HUVUi",
    "LCAiS3VldDF5QWEwSElRdlluT1ZkNTloY1ZpTzlVZzZKMmtTZnFZUkJlb3d2RSIs",
    "ICJNTWxkT0ZGekIyZDB1bWxtcFRJYUdlcmhXZFVfUHBZZkx2S2hoX2ZfOWFZIiwg",
    "Ilg2WkFZT0lJMnZQTjQwVjd4RXhad1Z3ejd5Um1MTmNWd3Q1REw4Ukx2NGciLCAi",
    "WTM0em1JbzBRTExPdGRNcFhHd2pCZ0x2cjE3eUVoaFlUMEZHb2ZSLWFJRSIsICJm",
    "eUdwMFdUd3dQdjJKRFFsbjFsU2lhZW9iWnNNV0ExMGJRNTk4OS05RFRzIiwgIm9t",
    "bUZBaWNWVDhMR0hDQjB1eXd4N2ZZdW8zTUhZS08xNWN6LVJaRVlNNVEiLCAiczBC",
    "S1lzTFd4UVFlVTh0VmxsdE03TUtzSVJUckVJYTFQa0ptcXhCQmY1VSJdLCAiaXNz",
    "IjogImh0dHBzOi8vaXNzdWVyLmV4YW1wbGUuY29tIiwgImlhdCI6IDE2ODMwMDAw",
    "MDAsICJleHAiOiAxODgzMDAwMDAwLCAiYWRkcmVzcyI6IHsiX3NkIjogWyI2YVVo",
    "elloWjdTSjFrVm1hZ1FBTzN1MkVUTjJDQzFhSGhlWnBLbmFGMF9FIiwgIkF6TGxG",
    "b2JrSjJ4aWF1cFJFUHlvSnotOS1OU2xkQjZDZ2pyN2ZVeW9IemciLCAiUHp6Y1Z1",
    "MHFiTXVCR1NqdWxmZXd6a2VzRDl6dXRPRXhuNUVXTndrclEtayIsICJiMkRrdzBq",
    "Y0lGOXJHZzhfUEY4WmN2bmNXN3p3Wmo1cnlCV3ZYZnJwemVrIiwgImNQWUpISVo4",
    "VnUtZjlDQ3lWdWIyVWZnRWs4anZ2WGV6d0sxcF9KbmVlWFEiLCAiZ2xUM2hyU1U3",
    "ZlNXZ3dGNVVEWm1Xd0JUdzMyZ25VbGRJaGk4aEdWQ2FWNCIsICJydkpkNmlxNlQ1",
    "ZWptc0JNb0d3dU5YaDlxQUFGQVRBY2k0MG9pZEVlVnNBIiwgInVOSG9XWWhYc1po",
    "VkpDTkUyRHF5LXpxdDd0NjlnSkt5NVFhRnY3R3JNWDQiXX0sICJfc2RfYWxnIjog",
    "InNoYS0yNTYifQ.",
    "EOZa2YqK8j4i7cqBDkfPcTMaFsgPwcx3aYJkFoMfvV46LxL-P",
    "PqrWsIyNukB4x8Y2LT31eIHDc4Wg4XNzaqu4w~WyJHMDJOU3JRZmpGWFE3SW8wOX",
    "N5YWpBIiwgInJlZ2lvbiIsICJcdTZlMmZcdTUzM2EiXQ~WyJsa2x4RjVqTVlsR1R",
    "QVW92TU5JdkNBIiwgImNvdW50cnkiLCAiSlAiXQ~",
);

/// Appendix A.1: the Processed SD-JWT Payload the RFC prints for it.
pub(super) const A1_PROCESSED: &str = r#"{
  "iss": "https://issuer.example.com",
  "iat": 1683000000,
  "exp": 1883000000,
  "address": {
    "region": "港区",
    "country": "JP"
  }
}"#;

/// Appendix A.3: the claims the Issuer is given.
pub(super) const A3_INPUT: &str = r#"{
  "vct": "urn:eudi:pid:de:1",
  "iss": "https://pid-issuer.bund.de.example",
  "given_name": "Erika",
  "family_name": "Mustermann",
  "birthdate": "1963-08-12",
  "address": {
    "street_address": "Heidestraße 17",
    "locality": "Köln",
    "postal_code": "51147",
    "country": "DE"
  },
  "nationalities": [
    "DE"
  ],
  "sex": 2,
  "birth_family_name": "Gabler",
  "place_of_birth": {
    "locality": "Berlin",
    "country": "DE"
  },
  "age_equal_or_over": {
    "12": true,
    "14": true,
    "16": true,
    "18": true,
    "21": true,
    "65": false
  },
  "age_in_years": 62,
  "age_birth_year": 1963,
  "issuance_date": "2020-03-11",
  "expiry_date": "2030-03-12",
  "issuing_authority": "DE",
  "issuing_country": "DE"
}"#;

/// Appendix A.3: the issued SD-JWT, every Disclosure included.
pub(super) const A3_ISSUED: &str = concat!(
    "eyJhbGciOiAiRVMyNTYiLCAidHlwIjogImRjK3NkLWp3dCJ9.",
    "eyJfc2QiOiBbIjB",
    "IWm1uU0lQejMzN2tTV2U3QzM0bC0tODhnekppLWVCSjJWel9ISndBVGciLCAiMUN",
    "ybjAzV21VZVJXcDR6d1B2dkNLWGw5WmFRcC1jZFFWX2dIZGFHU1dvdyIsICIycjA",
    "wOWR6dkh1VnJXclJYVDVrSk1tSG5xRUhIbldlME1MVlp3OFBBVEI4IiwgIjZaTkl",
    "TRHN0NjJ5bWxyT0FrYWRqZEQ1WnVsVDVBMjk5Sjc4U0xoTV9fT3MiLCAiNzhqZzc",
    "3LUdZQmVYOElRZm9FTFB5TDBEWVBkbWZabzBKZ1ZpVjBfbEtDTSIsICI5MENUOEF",
    "hQlBibjVYOG5SWGtlc2p1MWkwQnFoV3FaM3dxRDRqRi1xREdrIiwgIkkwMGZjRlV",
    "vRFhDdWNwNXl5MnVqcVBzc0RWR2FXTmlVbGlOel9hd0QwZ2MiLCAiS2pBWGdBQTl",
    "ONVdIRUR0UkloNHU1TW4xWnNXaXhoaFdBaVgtQTRRaXdnQSIsICJMYWk2SVU2ZDd",
    "HUWFnWFI3QXZHVHJuWGdTbGQzejhFSWdfZnYzZk9aMVdnIiwgIkxlemphYlJxaVp",
    "PWHpFWW1WWmY4Uk1pOXhBa2QzX00xTFo4VTdFNHMzdTQiLCAiUlR6M3FUbUZOSGJ",
    "wV3JyT01aUzQxRjQ3NGtGcVJ2M3ZJUHF0aDZQVWhsTSIsICJXMTRYSGJVZmZ6dVc",
    "0SUZNanBTVGIxbWVsV3hVV2Y0Tl9vMmxka2tJcWM4IiwgIldUcEk3UmNNM2d4WnJ",
    "1UnBYemV6U2JrYk9yOTNQVkZ2V3g4d29KM2oxY0UiLCAiX29oSlZJUUlCc1U0dXB",
    "kTlM0X3c0S2IxTUhxSjBMOXFMR3NoV3E2SlhRcyIsICJ5NTBjemMwSVNDaHlfYnN",
    "iYTFkTW9VdUFPUTVBTW1PU2ZHb0VlODF2MUZVIl0sICJpc3MiOiAiaHR0cHM6Ly9",
    "waWQtaXNzdWVyLmJ1bmQuZGUuZXhhbXBsZSIsICJpYXQiOiAxNjgzMDAwMDAwLCA",
    "iZXhwIjogMTg4MzAwMDAwMCwgInZjdCI6ICJ1cm46ZXVkaTpwaWQ6ZGU6MSIsICJ",
    "fc2RfYWxnIjogInNoYS0yNTYiLCAiY25mIjogeyJqd2siOiB7Imt0eSI6ICJFQyI",
    "sICJjcnYiOiAiUC0yNTYiLCAieCI6ICJUQ0FFUjE5WnZ1M09IRjRqNFc0dmZTVm9",
    "ISVAxSUxpbERsczd2Q2VHZW1jIiwgInkiOiAiWnhqaVdXYlpNUUdIVldLVlE0aGJ",
    "TSWlyc1ZmdWVjQ0U2dDRqVDlGMkhaUSJ9fX0.",
    "ZOZQTqmq8X1mCyFXi0wbV8xjctX",
    "1AlEa5TkdnkKOyWvLfW40XDb5oj9tzkgwff5s44IDnrfAdgLtmTcojs97_Q~WyIy",
    "R0xDNDJzS1F2ZUNmR2ZyeU5STjl3IiwgImdpdmVuX25hbWUiLCAiRXJpa2EiXQ~W",
    "yJlbHVWNU9nM2dTTklJOEVZbnN4QV9BIiwgImZhbWlseV9uYW1lIiwgIk11c3Rlc",
    "m1hbm4iXQ~WyI2SWo3dE0tYTVpVlBHYm9TNXRtdlZBIiwgImJpcnRoZGF0ZSIsIC",
    "IxOTYzLTA4LTEyIl0~WyJlSThaV205UW5LUHBOUGVOZW5IZGhRIiwgInN0cmVldF",
    "9hZGRyZXNzIiwgIkhlaWRlc3RyYVx1MDBkZmUgMTciXQ~WyJRZ19PNjR6cUF4ZTQ",
    "xMmExMDhpcm9BIiwgImxvY2FsaXR5IiwgIktcdTAwZjZsbiJd~WyJBSngtMDk1Vl",
    "BycFR0TjRRTU9xUk9BIiwgInBvc3RhbF9jb2RlIiwgIjUxMTQ3Il0~WyJQYzMzSk",
    "0yTGNoY1VfbEhnZ3ZfdWZRIiwgImNvdW50cnkiLCAiREUiXQ~WyJHMDJOU3JRZmp",
    "GWFE3SW8wOXN5YWpBIiwgImFkZHJlc3MiLCB7Il9zZCI6IFsiQUxaRVJzU241V05",
    "pRVhkQ2tzVzhJNXFRdzNfTnBBblJxcFNBWkR1ZGd3OCIsICJEX19XX3VZY3ZSejN",
    "0dlVuSUp2QkRIaVRjN0NfX3FIZDB4Tkt3SXNfdzlrIiwgImVCcENYVTFKNWRoSDJ",
    "nNHQ4UVlOVzVFeFM5QXhVVmJsVW9kb0xZb1BobzAiLCAieE9QeTktZ0pBTEs2VWJ",
    "XS0ZMUjg1Y09CeVVEM0FiTndGZzNJM1lmUUVfSSJdfV0~WyJsa2x4RjVqTVlsR1R",
    "QVW92TU5JdkNBIiwgIm5hdGlvbmFsaXRpZXMiLCBbIkRFIl1d~WyJuUHVvUW5rUk",
    "ZxM0JJZUFtN0FuWEZBIiwgInNleCIsIDJd~WyI1YlBzMUlxdVpOYTBoa2FGenp6W",
    "k53IiwgImJpcnRoX2ZhbWlseV9uYW1lIiwgIkdhYmxlciJd~WyI1YTJXMF9OcmxF",
    "WnpmcW1rXzdQcS13IiwgImxvY2FsaXR5IiwgIkJlcmxpbiJd~WyJ5MXNWVTV3ZGZ",
    "KYWhWZGd3UGdTN1JRIiwgImNvdW50cnkiLCAiREUiXQ~WyJIYlE0WDhzclZXM1FE",
    "eG5JSmRxeU9BIiwgInBsYWNlX29mX2JpcnRoIiwgeyJfc2QiOiBbIktVVmlhYUxu",
    "WTVqU01MOTBHMjlPT0xFTlBiYlhmaFNqU1BNalphR2t4QUUiLCAiWWJzVDBTNzZW",
    "cVhDVnNkMWpVU2x3S1BEZ21BTGVCMXVaY2xGSFhmLVVTUSJdfV0~WyJDOUdTb3Vq",
    "dmlKcXVFZ1lmb2pDYjFBIiwgIjEyIiwgdHJ1ZV0~WyJreDVrRjE3Vi14MEptd1V4",
    "OXZndnR3IiwgIjE0IiwgdHJ1ZV0~WyJIM28xdXN3UDc2MEZpMnllR2RWQ0VRIiwg",
    "IjE2IiwgdHJ1ZV0~WyJPQktsVFZsdkxnLUFkd3FZR2JQOFpBIiwgIjE4IiwgdHJ1",
    "ZV0~WyJNMEpiNTd0NDF1YnJrU3V5ckRUM3hBIiwgIjIxIiwgdHJ1ZV0~WyJEc210",
    "S05ncFY0ZEFIcGpyY2Fvc0F3IiwgIjY1IiwgZmFsc2Vd~WyJlSzVvNXBIZmd1cFB",
    "wbHRqMXFoQUp3IiwgImFnZV9lcXVhbF9vcl9vdmVyIiwgeyJfc2QiOiBbIjF0RWl",
    "5elBSWU9Lc2Y3U3NZR01nUFpLc09UMWxRWlJ4SFhBMHI1X0J3a2siLCAiQ1ZLbmx",
    "5NVA5MHlKczNFd3R4UWlPdFVjemFYQ1lOQTRJY3pSYW9ock1EZyIsICJhNDQtZzJ",
    "HcjhfM0FtSncyWFo4a0kxeTBRel96ZTlpT2NXMlczUkxwWEdnIiwgImdrdnkwRnV",
    "2QkJ2ajBoczJaTnd4Y3FPbGY4bXUyLWtDRTctTmIyUXh1QlUiLCAiaHJZNEhubUY",
    "1YjVKd0M5ZVR6YUZDVWNlSVFBYUlkaHJxVVhRTkNXYmZaSSIsICJ5NlNGclZGUnl",
    "xNTBJYlJKdmlUWnFxalFXejB0TGl1Q21NZU8wS3FhekdJIl19XQ~WyJqN0FEZGIw",
    "VVZiMExpMGNpUGNQMGV3IiwgImFnZV9pbl95ZWFycyIsIDYyXQ~WyJXcHhKckZ1W",
    "Dh1U2kycDRodDA5anZ3IiwgImFnZV9iaXJ0aF95ZWFyIiwgMTk2M10~WyJhdFNtR",
    "kFDWU1iSlZLRDA1bzNKZ3RRIiwgImlzc3VhbmNlX2RhdGUiLCAiMjAyMC0wMy0xM",
    "SJd~WyI0S3lSMzJvSVp0LXprV3ZGcWJVTEtnIiwgImV4cGlyeV9kYXRlIiwgIjIw",
    "MzAtMDMtMTIiXQ~WyJjaEJDc3loeWgtSjg2SS1hd1FEaUNRIiwgImlzc3VpbmdfY",
    "XV0aG9yaXR5IiwgIkRFIl0~WyJmbE5QMW5jTXo5TGctYzlxTUl6XzlnIiwgImlzc",
    "3VpbmdfY291bnRyeSIsICJERSJd~",
);

/// Appendix A.3: an SD-JWT+KB disclosing the nationality and that the holder is over 18. The last segment is the Key Binding JWT.
pub(super) const A3_PRESENTATION: &str = concat!(
    "eyJhbGciOiAiRVMyNTYiLCAidHlwIjogImRjK3NkLWp3dCJ9.",
    "eyJfc2QiOiBbIjB",
    "IWm1uU0lQejMzN2tTV2U3QzM0bC0tODhnekppLWVCSjJWel9ISndBVGciLCAiMUN",
    "ybjAzV21VZVJXcDR6d1B2dkNLWGw5WmFRcC1jZFFWX2dIZGFHU1dvdyIsICIycjA",
    "wOWR6dkh1VnJXclJYVDVrSk1tSG5xRUhIbldlME1MVlp3OFBBVEI4IiwgIjZaTkl",
    "TRHN0NjJ5bWxyT0FrYWRqZEQ1WnVsVDVBMjk5Sjc4U0xoTV9fT3MiLCAiNzhqZzc",
    "3LUdZQmVYOElRZm9FTFB5TDBEWVBkbWZabzBKZ1ZpVjBfbEtDTSIsICI5MENUOEF",
    "hQlBibjVYOG5SWGtlc2p1MWkwQnFoV3FaM3dxRDRqRi1xREdrIiwgIkkwMGZjRlV",
    "vRFhDdWNwNXl5MnVqcVBzc0RWR2FXTmlVbGlOel9hd0QwZ2MiLCAiS2pBWGdBQTl",
    "ONVdIRUR0UkloNHU1TW4xWnNXaXhoaFdBaVgtQTRRaXdnQSIsICJMYWk2SVU2ZDd",
    "HUWFnWFI3QXZHVHJuWGdTbGQzejhFSWdfZnYzZk9aMVdnIiwgIkxlemphYlJxaVp",
    "PWHpFWW1WWmY4Uk1pOXhBa2QzX00xTFo4VTdFNHMzdTQiLCAiUlR6M3FUbUZOSGJ",
    "wV3JyT01aUzQxRjQ3NGtGcVJ2M3ZJUHF0aDZQVWhsTSIsICJXMTRYSGJVZmZ6dVc",
    "0SUZNanBTVGIxbWVsV3hVV2Y0Tl9vMmxka2tJcWM4IiwgIldUcEk3UmNNM2d4WnJ",
    "1UnBYemV6U2JrYk9yOTNQVkZ2V3g4d29KM2oxY0UiLCAiX29oSlZJUUlCc1U0dXB",
    "kTlM0X3c0S2IxTUhxSjBMOXFMR3NoV3E2SlhRcyIsICJ5NTBjemMwSVNDaHlfYnN",
    "iYTFkTW9VdUFPUTVBTW1PU2ZHb0VlODF2MUZVIl0sICJpc3MiOiAiaHR0cHM6Ly9",
    "waWQtaXNzdWVyLmJ1bmQuZGUuZXhhbXBsZSIsICJpYXQiOiAxNjgzMDAwMDAwLCA",
    "iZXhwIjogMTg4MzAwMDAwMCwgInZjdCI6ICJ1cm46ZXVkaTpwaWQ6ZGU6MSIsICJ",
    "fc2RfYWxnIjogInNoYS0yNTYiLCAiY25mIjogeyJqd2siOiB7Imt0eSI6ICJFQyI",
    "sICJjcnYiOiAiUC0yNTYiLCAieCI6ICJUQ0FFUjE5WnZ1M09IRjRqNFc0dmZTVm9",
    "ISVAxSUxpbERsczd2Q2VHZW1jIiwgInkiOiAiWnhqaVdXYlpNUUdIVldLVlE0aGJ",
    "TSWlyc1ZmdWVjQ0U2dDRqVDlGMkhaUSJ9fX0.",
    "ZOZQTqmq8X1mCyFXi0wbV8xjctX",
    "1AlEa5TkdnkKOyWvLfW40XDb5oj9tzkgwff5s44IDnrfAdgLtmTcojs97_Q~WyJl",
    "SzVvNXBIZmd1cFBwbHRqMXFoQUp3IiwgImFnZV9lcXVhbF9vcl9vdmVyIiwgeyJf",
    "c2QiOiBbIjF0RWl5elBSWU9Lc2Y3U3NZR01nUFpLc09UMWxRWlJ4SFhBMHI1X0J3",
    "a2siLCAiQ1ZLbmx5NVA5MHlKczNFd3R4UWlPdFVjemFYQ1lOQTRJY3pSYW9ock1E",
    "ZyIsICJhNDQtZzJHcjhfM0FtSncyWFo4a0kxeTBRel96ZTlpT2NXMlczUkxwWEdn",
    "IiwgImdrdnkwRnV2QkJ2ajBoczJaTnd4Y3FPbGY4bXUyLWtDRTctTmIyUXh1QlUi",
    "LCAiaHJZNEhubUY1YjVKd0M5ZVR6YUZDVWNlSVFBYUlkaHJxVVhRTkNXYmZaSSIs",
    "ICJ5NlNGclZGUnlxNTBJYlJKdmlUWnFxalFXejB0TGl1Q21NZU8wS3FhekdJIl19",
    "XQ~WyJPQktsVFZsdkxnLUFkd3FZR2JQOFpBIiwgIjE4IiwgdHJ1ZV0~WyJsa2x4R",
    "jVqTVlsR1RQVW92TU5JdkNBIiwgIm5hdGlvbmFsaXRpZXMiLCBbIkRFIl1d~eyJh",
    "bGciOiAiRVMyNTYiLCAidHlwIjogImtiK2p3dCJ9.",
    "eyJub25jZSI6ICIxMjM0NTY",
    "3ODkwIiwgImF1ZCI6ICJodHRwczovL3ZlcmlmaWVyLmV4YW1wbGUub3JnIiwgIml",
    "hdCI6IDE3NDg1MzcyNDQsICJzZF9oYXNoIjogIlBqTVlmTTA3VmJKZE14TElsdXZ",
    "STmI4OEpGbGpTWDRuLUc0M1VjX0JTUk0ifQ.",
    "f3TeS_1BWEG78EbIJRh5wgv8nYum",
    "k7euzu6xgbgpNB4pbQQqgRPWK-vQjlhhgU1EFGZ9LFakFX_0mgul1G_3mw",
);

/// Appendix A.3: the Processed SD-JWT Payload the RFC prints for it.
pub(super) const A3_PROCESSED: &str = r#"{
  "iss": "https://pid-issuer.bund.de.example",
  "iat": 1683000000,
  "exp": 1883000000,
  "vct": "urn:eudi:pid:de:1",
  "cnf": {
    "jwk": {
      "kty": "EC",
      "crv": "P-256",
      "x": "TCAER19Zvu3OHF4j4W4vfSVoHIP1ILilDls7vCeGemc",
      "y": "ZxjiWWbZMQGHVWKVQ4hbSIirsVfuecCE6t4jT9F2HZQ"
    }
  },
  "age_equal_or_over": {
    "18": true
  },
  "nationalities": [
    "DE"
  ]
}"#;

/// Appendix A.4: an SD-JWT+KB over a W3C credential payload. The last segment is the Key Binding JWT.
pub(super) const A4_PRESENTATION: &str = concat!(
    "eyJhbGciOiAiRVMyNTYiLCAidHlwIjogImV4YW1wbGUrc2Qtand0In0.",
    "eyJAY29u",
    "dGV4dCI6IFsiaHR0cHM6Ly93d3cudzMub3JnLzIwMTgvY3JlZGVudGlhbHMvdjEi",
    "LCAiaHR0cHM6Ly93M2lkLm9yZy92YWNjaW5hdGlvbi92MSJdLCAidHlwZSI6IFsi",
    "VmVyaWZpYWJsZUNyZWRlbnRpYWwiLCAiVmFjY2luYXRpb25DZXJ0aWZpY2F0ZSJd",
    "LCAiaXNzdWVyIjogImh0dHBzOi8vZXhhbXBsZS5jb20vaXNzdWVyIiwgImlzc3Vh",
    "bmNlRGF0ZSI6ICIyMDIzLTAyLTA5VDExOjAxOjU5WiIsICJleHBpcmF0aW9uRGF0",
    "ZSI6ICIyMDI4LTAyLTA4VDExOjAxOjU5WiIsICJuYW1lIjogIkNPVklELTE5IFZh",
    "Y2NpbmF0aW9uIENlcnRpZmljYXRlIiwgImRlc2NyaXB0aW9uIjogIkNPVklELTE5",
    "IFZhY2NpbmF0aW9uIENlcnRpZmljYXRlIiwgImNyZWRlbnRpYWxTdWJqZWN0Ijog",
    "eyJfc2QiOiBbIjFWX0stOGxEUThpRlhCRlhiWlk5ZWhxUjRIYWJXQ2k1VDB5Ykl6",
    "WlBld3ciLCAiSnpqTGd0UDI5ZFAtQjN0ZDEyUDY3NGdGbUsyenk4MUhNdEJnZjZD",
    "Sk5XZyIsICJSMmZHYmZBMDdaX1lsa3FtTlp5bWExeHl5eDFYc3RJaVM2QjFZYmwy",
    "Slo0IiwgIlRDbXpybDdLMmdldl9kdTdwY01JeXpSTEhwLVllZy1GbF9jeHRyVXZQ",
    "eGciLCAiVjdrSkJMSzc4VG1WRE9tcmZKN1p1VVBIdUtfMmNjN3laUmE0cVYxdHh3",
    "TSIsICJiMGVVc3ZHUC1PRERkRm9ZNE5semxYYzN0RHNsV0p0Q0pGNzVOdzhPal9n",
    "IiwgInpKS19lU01YandNOGRYbU1aTG5JOEZHTTA4ekozX3ViR2VFTUotNVRCeTAi",
    "XSwgInZhY2NpbmUiOiB7Il9zZCI6IFsiMWNGNWhMd2toTU5JYXFmV0pyWEk3Tk1X",
    "ZWRMLTlmNlkyUEE1MnlQalNaSSIsICJIaXk2V1d1ZUxENWJuMTYyOTh0UHY3R1ho",
    "bWxkTURPVG5CaS1DWmJwaE5vIiwgIkxiMDI3cTY5MWpYWGwtakM3M3ZpOGViT2o5",
    "c214M0MtX29nN2dBNFRCUUUiXSwgInR5cGUiOiAiVmFjY2luZSJ9LCAicmVjaXBp",
    "ZW50IjogeyJfc2QiOiBbIjFsU1FCTlkyNHEwVGg2T0d6dGhxLTctNGw2Y0FheHJZ",
    "WE9HWnBlV19sbkEiLCAiM256THE4MU0yb04wNndkdjFzaEh2T0VKVnhaNUtMbWRE",
    "a0hFREpBQldFSSIsICJQbjFzV2kwNkc0TEpybm4tX1JUMFJiTV9IVGR4blBKUXVY",
    "MmZ6V3ZfSk9VIiwgImxGOXV6ZHN3N0hwbEdMYzcxNFRyNFdPN01HSnphN3R0N1FG",
    "bGVDWDRJdHciXSwgInR5cGUiOiAiVmFjY2luZVJlY2lwaWVudCJ9LCAidHlwZSI6",
    "ICJWYWNjaW5hdGlvbkV2ZW50In0sICJfc2RfYWxnIjogInNoYS0yNTYiLCAiY25m",
    "IjogeyJqd2siOiB7Imt0eSI6ICJFQyIsICJjcnYiOiAiUC0yNTYiLCAieCI6ICJU",
    "Q0FFUjE5WnZ1M09IRjRqNFc0dmZTVm9ISVAxSUxpbERsczd2Q2VHZW1jIiwgInki",
    "OiAiWnhqaVdXYlpNUUdIVldLVlE0aGJTSWlyc1ZmdWVjQ0U2dDRqVDlGMkhaUSJ9",
    "fX0.OZomvwO8iw4db89MYCeeomBVStXkT6u7G7FkicPWZnd2_hGgr0l_u1NHgPVo",
    "cuOt-m32Uu6kwtPmYFxKk0AOeA~WyJQYzMzSk0yTGNoY1VfbEhnZ3ZfdWZRIiwgI",
    "m9yZGVyIiwgIjMvMyJd~WyJBSngtMDk1VlBycFR0TjRRTU9xUk9BIiwgImRhdGVP",
    "ZlZhY2NpbmF0aW9uIiwgIjIwMjEtMDYtMjNUMTM6NDA6MTJaIl0~WyIyR0xDNDJz",
    "S1F2ZUNmR2ZyeU5STjl3IiwgImF0Y0NvZGUiLCAiSjA3QlgwMyJd~WyJlbHVWNU9",
    "nM2dTTklJOEVZbnN4QV9BIiwgIm1lZGljaW5hbFByb2R1Y3ROYW1lIiwgIkNPVkl",
    "ELTE5IFZhY2NpbmUgTW9kZXJuYSJd~eyJhbGciOiAiRVMyNTYiLCAidHlwIjogIm",
    "tiK2p3dCJ9.",
    "eyJub25jZSI6ICIxMjM0NTY3ODkwIiwgImF1ZCI6ICJodHRwczovL",
    "3ZlcmlmaWVyLmV4YW1wbGUub3JnIiwgImlhdCI6IDE3NDg1MzcyNDQsICJzZF9oY",
    "XNoIjogIklvV1VIOTFsbGYzWEVybDQyYlEzc3hfNTNWMW8xdWpDejA4aERxSEs3R",
    "GsifQ.n0vzyIwCFMDVauEaeJIWEKZZchxXMpXTQewHgAkARbOSZxB09IbXXtHfpo",
    "GqO_BtNFN2lShJEIQBGyc-XpHigA",
);

/// Appendix A.4: the Processed SD-JWT Payload the RFC prints for it.
pub(super) const A4_PROCESSED: &str = r#"{
  "@context": [
    "https://www.w3.org/2018/credentials/v1",
    "https://w3id.org/vaccination/v1"
  ],
  "type": [
    "VerifiableCredential",
    "VaccinationCertificate"
  ],
  "issuer": "https://example.com/issuer",
  "issuanceDate": "2023-02-09T11:01:59Z",
  "expirationDate": "2028-02-08T11:01:59Z",
  "name": "COVID-19 Vaccination Certificate",
  "description": "COVID-19 Vaccination Certificate",
  "credentialSubject": {
    "vaccine": {
      "type": "Vaccine",
      "atcCode": "J07BX03",
      "medicinalProductName": "COVID-19 Vaccine Moderna"
    },
    "recipient": {
      "type": "VaccineRecipient"
    },
    "type": "VaccinationEvent",
    "order": "3/3",
    "dateOfVaccination": "2021-06-23T13:40:12Z"
  },
  "cnf": {
    "jwk": {
      "kty": "EC",
      "crv": "P-256",
      "x": "TCAER19Zvu3OHF4j4W4vfSVoHIP1ILilDls7vCeGemc",
      "y": "ZxjiWWbZMQGHVWKVQ4hbSIirsVfuecCE6t4jT9F2HZQ"
    }
  }
}"#;

/// Section 6.1: the Issuer-signed JWT's payload, with the address as one Disclosure.
pub(super) const S61_PAYLOAD: &str = r#"{
  "_sd": [
    "fOBUSQvo46yQO-wRwXBcGqvnbKIueISEL961_Sjd4do"
  ],
  "iss": "https://issuer.example.com",
  "iat": 1683000000,
  "exp": 1883000000,
  "sub": "6c5c0a49-b589-431d-bae7-219122a9ec2c",
  "_sd_alg": "sha-256"
}"#;

pub(super) const S61_ADDRESS: (&str, &str) = (
    "fOBUSQvo46yQO-wRwXBcGqvnbKIueISEL961_Sjd4do",
    concat!(
        "WyIyR0xDNDJzS1F2ZUNmR2ZyeU5STjl3IiwgImFkZHJlc3MiLCB7InN0cmVldF9h",
        "ZGRyZXNzIjogIlNjaHVsc3RyLiAxMiIsICJsb2NhbGl0eSI6ICJTY2h1bHBmb3J0",
        "YSIsICJyZWdpb24iOiAiU2FjaHNlbi1BbmhhbHQiLCAiY291bnRyeSI6ICJERSJ9",
        "XQ",
    ),
);

/// Section 6.2: the address's own members are each a Disclosure.
pub(super) const S62_PAYLOAD: &str = r#"{
  "iss": "https://issuer.example.com",
  "iat": 1683000000,
  "exp": 1883000000,
  "sub": "6c5c0a49-b589-431d-bae7-219122a9ec2c",
  "address": {
    "_sd": [
      "6vh9bq-zS4GKM_7GpggVbYzzu6oOGXrmNVGPHP75Ud0",
      "9gjVuXtdFROCgRrtNcGUXmF65rdezi_6Er_j76kmYyM",
      "KURDPh4ZC19-3tiz-Df39V8eidy1oV3a3H1Da2N0g88",
      "WN9r9dCBJ8HTCsS2jKASxTjEyW5m5x65_Z_2ro2jfXM"
    ]
  },
  "_sd_alg": "sha-256"
}"#;

/// Section 6.2: the same, with `country` left in the clear and no Disclosure for it.
pub(super) const S62_PAYLOAD_COUNTRY_IN_THE_CLEAR: &str = r#"{
  "iss": "https://issuer.example.com",
  "iat": 1683000000,
  "exp": 1883000000,
  "sub": "6c5c0a49-b589-431d-bae7-219122a9ec2c",
  "address": {
    "_sd": [
      "6vh9bq-zS4GKM_7GpggVbYzzu6oOGXrmNVGPHP75Ud0",
      "9gjVuXtdFROCgRrtNcGUXmF65rdezi_6Er_j76kmYyM",
      "KURDPh4ZC19-3tiz-Df39V8eidy1oV3a3H1Da2N0g88"
    ],
    "country": "DE"
  },
  "_sd_alg": "sha-256"
}"#;

pub(super) const S62_DISCLOSURES: [(&str, &str, &str); 4] = [
    (
        "street_address",
        "9gjVuXtdFROCgRrtNcGUXmF65rdezi_6Er_j76kmYyM",
        concat!(
            "WyIyR0xDNDJzS1F2ZUNmR2ZyeU5STjl3IiwgInN0cmVldF9hZGRyZXNzIiwgIlNj",
            "aHVsc3RyLiAxMiJd",
        ),
    ),
    (
        "locality",
        "6vh9bq-zS4GKM_7GpggVbYzzu6oOGXrmNVGPHP75Ud0",
        concat!(
            "WyJlbHVWNU9nM2dTTklJOEVZbnN4QV9BIiwgImxvY2FsaXR5IiwgIlNjaHVscGZv",
            "cnRhIl0",
        ),
    ),
    (
        "region",
        "KURDPh4ZC19-3tiz-Df39V8eidy1oV3a3H1Da2N0g88",
        concat!(
            "WyI2SWo3dE0tYTVpVlBHYm9TNXRtdlZBIiwgInJlZ2lvbiIsICJTYWNoc2VuLUFu",
            "aGFsdCJd",
        ),
    ),
    (
        "country",
        "WN9r9dCBJ8HTCsS2jKASxTjEyW5m5x65_Z_2ro2jfXM",
        "WyJlSThaV205UW5LUHBOUGVOZW5IZGhRIiwgImNvdW50cnkiLCAiREUiXQ",
    ),
];

/// Section 6.3: the address is a Disclosure whose value holds the digests of its own members.
pub(super) const S63_PAYLOAD: &str = r#"{
  "_sd": [
    "HvrKX6fPV0v9K_yCVFBiLFHsMaxcD_114Em6VT8x1lg"
  ],
  "iss": "https://issuer.example.com",
  "iat": 1683000000,
  "exp": 1883000000,
  "sub": "6c5c0a49-b589-431d-bae7-219122a9ec2c",
  "_sd_alg": "sha-256"
}"#;

/// Section 6.3: the address Disclosure, whose value holds the digests of its own four members,
/// which are the four Disclosures of section 6.2.
pub(super) const S63_ADDRESS: (&str, &str) = (
    "HvrKX6fPV0v9K_yCVFBiLFHsMaxcD_114Em6VT8x1lg",
    concat!(
        "WyJRZ19PNjR6cUF4ZTQxMmExMDhpcm9BIiwgImFkZHJlc3MiLCB7Il9zZCI6IFsi",
        "NnZoOWJxLXpTNEdLTV83R3BnZ1ZiWXp6dTZvT0dYcm1OVkdQSFA3NVVkMCIsICI5",
        "Z2pWdVh0ZEZST0NnUnJ0TmNHVVhtRjY1cmRlemlfNkVyX2o3NmttWXlNIiwgIktV",
        "UkRQaDRaQzE5LTN0aXotRGYzOVY4ZWlkeTFvVjNhM0gxRGEyTjBnODgiLCAiV045",
        "cjlkQ0JKOEhUQ3NTMmpLQVN4VGpFeVc1bTV4NjVfWl8ycm8yamZYTSJdfV0",
    ),
);

/// Section 5.1: the claims the Issuer is given. The nationalities array is visible and its elements are hidden.
pub(super) const S51_INPUT: &str = r#"{
  "sub": "user_42",
  "given_name": "John",
  "family_name": "Doe",
  "email": "johndoe@example.com",
  "phone_number": "+1-202-555-0101",
  "phone_number_verified": true,
  "address": {
    "street_address": "123 Main St",
    "locality": "Anytown",
    "region": "Anystate",
    "country": "US"
  },
  "birthdate": "1940-01-01",
  "updated_at": 1570000000,
  "nationalities": [
    "US",
    "DE"
  ]
}"#;

/// Section 5.1: the issued SD-JWT, every Disclosure included, two of them for array elements.
pub(super) const S51_ISSUED: &str = concat!(
    "eyJhbGciOiAiRVMyNTYiLCAidHlwIjogImV4YW1wbGUrc2Qtand0In0.",
    "eyJfc2Qi",
    "OiBbIkNyUWU3UzVrcUJBSHQtbk1ZWGdjNmJkdDJTSDVhVFkxc1VfTS1QZ2tqUEki",
    "LCAiSnpZakg0c3ZsaUgwUjNQeUVNZmVadTZKdDY5dTVxZWhabzdGN0VQWWxTRSIs",
    "ICJQb3JGYnBLdVZ1Nnh5bUphZ3ZrRnNGWEFiUm9jMkpHbEFVQTJCQTRvN2NJIiwg",
    "IlRHZjRvTGJnd2Q1SlFhSHlLVlFaVTlVZEdFMHc1cnREc3JaemZVYW9tTG8iLCAi",
    "WFFfM2tQS3QxWHlYN0tBTmtxVlI2eVoyVmE1TnJQSXZQWWJ5TXZSS0JNTSIsICJY",
    "ekZyendzY002R242Q0pEYzZ2Vks4QmtNbmZHOHZPU0tmcFBJWmRBZmRFIiwgImdi",
    "T3NJNEVkcTJ4Mkt3LXc1d1BFemFrb2I5aFYxY1JEMEFUTjNvUUw5Sk0iLCAianN1",
    "OXlWdWx3UVFsaEZsTV8zSmx6TWFTRnpnbGhRRzBEcGZheVF3TFVLNCJdLCAiaXNz",
    "IjogImh0dHBzOi8vaXNzdWVyLmV4YW1wbGUuY29tIiwgImlhdCI6IDE2ODMwMDAw",
    "MDAsICJleHAiOiAxODgzMDAwMDAwLCAic3ViIjogInVzZXJfNDIiLCAibmF0aW9u",
    "YWxpdGllcyI6IFt7Ii4uLiI6ICJwRm5kamtaX1ZDem15VGE2VWpsWm8zZGgta284",
    "YUlLUWM5RGxHemhhVllvIn0sIHsiLi4uIjogIjdDZjZKa1B1ZHJ5M2xjYndIZ2Va",
    "OGtoQXYxVTFPU2xlclAwVmtCSnJXWjAifV0sICJfc2RfYWxnIjogInNoYS0yNTYi",
    "LCAiY25mIjogeyJqd2siOiB7Imt0eSI6ICJFQyIsICJjcnYiOiAiUC0yNTYiLCAi",
    "eCI6ICJUQ0FFUjE5WnZ1M09IRjRqNFc0dmZTVm9ISVAxSUxpbERsczd2Q2VHZW1j",
    "IiwgInkiOiAiWnhqaVdXYlpNUUdIVldLVlE0aGJTSWlyc1ZmdWVjQ0U2dDRqVDlG",
    "MkhaUSJ9fX0.",
    "MczwjBFGtzf-6WMT-hIvYbkb11NrV1WMO-jTijpMPNbswNzZ87wY",
    "2uHz-CXo6R04b7jYrpj9mNRAvVssXou1iw~WyIyR0xDNDJzS1F2ZUNmR2ZyeU5ST",
    "jl3IiwgImdpdmVuX25hbWUiLCAiSm9obiJd~WyJlbHVWNU9nM2dTTklJOEVZbnN4",
    "QV9BIiwgImZhbWlseV9uYW1lIiwgIkRvZSJd~WyI2SWo3dE0tYTVpVlBHYm9TNXR",
    "tdlZBIiwgImVtYWlsIiwgImpvaG5kb2VAZXhhbXBsZS5jb20iXQ~WyJlSThaV205",
    "UW5LUHBOUGVOZW5IZGhRIiwgInBob25lX251bWJlciIsICIrMS0yMDItNTU1LTAx",
    "MDEiXQ~WyJRZ19PNjR6cUF4ZTQxMmExMDhpcm9BIiwgInBob25lX251bWJlcl92Z",
    "XJpZmllZCIsIHRydWVd~WyJBSngtMDk1VlBycFR0TjRRTU9xUk9BIiwgImFkZHJl",
    "c3MiLCB7InN0cmVldF9hZGRyZXNzIjogIjEyMyBNYWluIFN0IiwgImxvY2FsaXR5",
    "IjogIkFueXRvd24iLCAicmVnaW9uIjogIkFueXN0YXRlIiwgImNvdW50cnkiOiAi",
    "VVMifV0~WyJQYzMzSk0yTGNoY1VfbEhnZ3ZfdWZRIiwgImJpcnRoZGF0ZSIsICIx",
    "OTQwLTAxLTAxIl0~WyJHMDJOU3JRZmpGWFE3SW8wOXN5YWpBIiwgInVwZGF0ZWRf",
    "YXQiLCAxNTcwMDAwMDAwXQ~WyJsa2x4RjVqTVlsR1RQVW92TU5JdkNBIiwgIlVTI",
    "l0~WyJuUHVvUW5rUkZxM0JJZUFtN0FuWEZBIiwgIkRFIl0~",
);

/// Section 5.1: the digest the RFC prints for each array element, and its Disclosure, in array order.
pub(super) const S51_ARRAY_DISCLOSURES: [(&str, &str); 2] = [
    (
        "pFndjkZ_VCzmyTa6UjlZo3dh-ko8aIKQc9DlGzhaVYo",
        "WyJsa2x4RjVqTVlsR1RQVW92TU5JdkNBIiwgIlVTIl0",
    ),
    (
        "7Cf6JkPudry3lcbwHgeZ8khAv1U1OSlerP0VkBJrWZ0",
        "WyJuUHVvUW5rUkZxM0JJZUFtN0FuWEZBIiwgIkRFIl0",
    ),
];

/// Section 5.2: an SD-JWT+KB disclosing given_name, family_name, address and the first nationality. The last segment is the Key Binding JWT.
pub(super) const S52_PRESENTATION: &str = concat!(
    "eyJhbGciOiAiRVMyNTYiLCAidHlwIjogImV4YW1wbGUrc2Qtand0In0.",
    "eyJfc2Qi",
    "OiBbIkNyUWU3UzVrcUJBSHQtbk1ZWGdjNmJkdDJTSDVhVFkxc1VfTS1QZ2tqUEki",
    "LCAiSnpZakg0c3ZsaUgwUjNQeUVNZmVadTZKdDY5dTVxZWhabzdGN0VQWWxTRSIs",
    "ICJQb3JGYnBLdVZ1Nnh5bUphZ3ZrRnNGWEFiUm9jMkpHbEFVQTJCQTRvN2NJIiwg",
    "IlRHZjRvTGJnd2Q1SlFhSHlLVlFaVTlVZEdFMHc1cnREc3JaemZVYW9tTG8iLCAi",
    "WFFfM2tQS3QxWHlYN0tBTmtxVlI2eVoyVmE1TnJQSXZQWWJ5TXZSS0JNTSIsICJY",
    "ekZyendzY002R242Q0pEYzZ2Vks4QmtNbmZHOHZPU0tmcFBJWmRBZmRFIiwgImdi",
    "T3NJNEVkcTJ4Mkt3LXc1d1BFemFrb2I5aFYxY1JEMEFUTjNvUUw5Sk0iLCAianN1",
    "OXlWdWx3UVFsaEZsTV8zSmx6TWFTRnpnbGhRRzBEcGZheVF3TFVLNCJdLCAiaXNz",
    "IjogImh0dHBzOi8vaXNzdWVyLmV4YW1wbGUuY29tIiwgImlhdCI6IDE2ODMwMDAw",
    "MDAsICJleHAiOiAxODgzMDAwMDAwLCAic3ViIjogInVzZXJfNDIiLCAibmF0aW9u",
    "YWxpdGllcyI6IFt7Ii4uLiI6ICJwRm5kamtaX1ZDem15VGE2VWpsWm8zZGgta284",
    "YUlLUWM5RGxHemhhVllvIn0sIHsiLi4uIjogIjdDZjZKa1B1ZHJ5M2xjYndIZ2Va",
    "OGtoQXYxVTFPU2xlclAwVmtCSnJXWjAifV0sICJfc2RfYWxnIjogInNoYS0yNTYi",
    "LCAiY25mIjogeyJqd2siOiB7Imt0eSI6ICJFQyIsICJjcnYiOiAiUC0yNTYiLCAi",
    "eCI6ICJUQ0FFUjE5WnZ1M09IRjRqNFc0dmZTVm9ISVAxSUxpbERsczd2Q2VHZW1j",
    "IiwgInkiOiAiWnhqaVdXYlpNUUdIVldLVlE0aGJTSWlyc1ZmdWVjQ0U2dDRqVDlG",
    "MkhaUSJ9fX0.",
    "MczwjBFGtzf-6WMT-hIvYbkb11NrV1WMO-jTijpMPNbswNzZ87wY",
    "2uHz-CXo6R04b7jYrpj9mNRAvVssXou1iw~WyJlbHVWNU9nM2dTTklJOEVZbnN4Q",
    "V9BIiwgImZhbWlseV9uYW1lIiwgIkRvZSJd~WyJBSngtMDk1VlBycFR0TjRRTU9x",
    "Uk9BIiwgImFkZHJlc3MiLCB7InN0cmVldF9hZGRyZXNzIjogIjEyMyBNYWluIFN0",
    "IiwgImxvY2FsaXR5IjogIkFueXRvd24iLCAicmVnaW9uIjogIkFueXN0YXRlIiwg",
    "ImNvdW50cnkiOiAiVVMifV0~WyIyR0xDNDJzS1F2ZUNmR2ZyeU5STjl3IiwgImdp",
    "dmVuX25hbWUiLCAiSm9obiJd~WyJsa2x4RjVqTVlsR1RQVW92TU5JdkNBIiwgIlV",
    "TIl0~eyJhbGciOiAiRVMyNTYiLCAidHlwIjogImtiK2p3dCJ9.",
    "eyJub25jZSI6IC",
    "IxMjM0NTY3ODkwIiwgImF1ZCI6ICJodHRwczovL3ZlcmlmaWVyLmV4YW1wbGUub3",
    "JnIiwgImlhdCI6IDE3NDg1MzcyNDQsICJzZF9oYXNoIjogIjBfQWYtMkItRWhMV1",
    "g1eWRoX3cyeHp3bU82aU02NkJfMlFDRWFuSTRmVVkifQ.",
    "T3SIus2OidNl41nmVkT",
    "ZVCKKhOAX97aOldMyHFiYjHm261eLiJ1YiuONFiMN8QlCmYzDlBLAdPvrXh52KaL",
    "gUQ",
);

/// Section 5.2: the Processed SD-JWT Payload the RFC prints for it. The second nationality is gone, not a placeholder.
pub(super) const S52_PROCESSED: &str = r#"{
  "iss": "https://issuer.example.com",
  "iat": 1683000000,
  "exp": 1883000000,
  "sub": "user_42",
  "nationalities": [
    "US"
  ],
  "cnf": {
    "jwk": {
      "kty": "EC",
      "crv": "P-256",
      "x": "TCAER19Zvu3OHF4j4W4vfSVoHIP1ILilDls7vCeGemc",
      "y": "ZxjiWWbZMQGHVWKVQ4hbSIirsVfuecCE6t4jT9F2HZQ"
    }
  },
  "family_name": "Doe",
  "address": {
    "street_address": "123 Main St",
    "locality": "Anytown",
    "region": "Anystate",
    "country": "US"
  },
  "given_name": "John"
}"#;

/// Appendix A.2: a presentation of a complex structure, which discloses one array element whose value holds further digests.
pub(super) const A2_PRESENTATION: &str = concat!(
    "eyJhbGciOiAiRVMyNTYiLCAidHlwIjogImV4YW1wbGUrc2Qtand0In0.",
    "eyJfc2Qi",
    "OiBbIi1hU3puSWQ5bVdNOG9jdVFvbENsbHN4VmdncTEtdkhXNE90bmhVdFZtV3ci",
    "LCAiSUticllObjN2QTdXRUZyeXN2YmRCSmpERFVfRXZRSXIwVzE4dlRScFVTZyIs",
    "ICJvdGt4dVQxNG5CaXd6TkozTVBhT2l0T2w5cFZuWE9hRUhhbF94a3lOZktJIl0s",
    "ICJpc3MiOiAiaHR0cHM6Ly9pc3N1ZXIuZXhhbXBsZS5jb20iLCAiaWF0IjogMTY4",
    "MzAwMDAwMCwgImV4cCI6IDE4ODMwMDAwMDAsICJ2ZXJpZmllZF9jbGFpbXMiOiB7",
    "InZlcmlmaWNhdGlvbiI6IHsiX3NkIjogWyI3aDRVRTlxU2N2REtvZFhWQ3VvS2ZL",
    "QkpwVkJmWE1GX1RtQUdWYVplM1NjIiwgInZUd2UzcmFISUZZZ0ZBM3hhVUQyYU14",
    "Rno1b0RvOGlCdTA1cUtsT2c5THciXSwgInRydXN0X2ZyYW1ld29yayI6ICJkZV9h",
    "bWwiLCAiZXZpZGVuY2UiOiBbeyIuLi4iOiAidFlKMFREdWN5WlpDUk1iUk9HNHFS",
    "TzV2a1BTRlJ4RmhVRUxjMThDU2wzayJ9XX0sICJjbGFpbXMiOiB7Il9zZCI6IFsi",
    "UmlPaUNuNl93NVpIYWFka1FNcmNRSmYwSnRlNVJ3dXJSczU0MjMxRFRsbyIsICJT",
    "XzQ5OGJicEt6QjZFYW5mdHNzMHhjN2NPYW9uZVJyM3BLcjdOZFJtc01vIiwgIldO",
    "QS1VTks3Rl96aHNBYjlzeVdPNklJUTF1SGxUbU9VOHI4Q3ZKMGNJTWsiLCAiV3ho",
    "X3NWM2lSSDliZ3JUQkppLWFZSE5DTHQtdmpoWDFzZC1pZ09mXzlsayIsICJfTy13",
    "SmlIM2VuU0I0Uk9IbnRUb1FUOEptTHR6LW1oTzJmMWM4OVhvZXJRIiwgImh2RFho",
    "d21HY0pRc0JDQTJPdGp1TEFjd0FNcERzYVUwbmtvdmNLT3FXTkUiXX19LCAiX3Nk",
    "X2FsZyI6ICJzaGEtMjU2In0.",
    "QoWYWtikm-AtjmPnNVshbGXQl5raEz15PByTmZwf",
    "TQg9W2O3oR6j2tMmysTZZawdo6mNLR_PsZSI25qrUpiNTg~WyIyR0xDNDJzS1F2Z",
    "UNmR2ZyeU5STjl3IiwgInRpbWUiLCAiMjAxMi0wNC0yM1QxODoyNVoiXQ~WyJQYz",
    "MzSk0yTGNoY1VfbEhnZ3ZfdWZRIiwgeyJfc2QiOiBbIjl3cGpWUFd1RDdQSzBuc1",
    "FETDhCMDZsbWRnVjNMVnliaEh5ZFFwVE55TEkiLCAiRzVFbmhPQU9vVTlYXzZRTU",
    "52ekZYanBFQV9SYy1BRXRtMWJHX3djYUtJayIsICJJaHdGcldVQjYzUmNacTl5dm",
    "daMFhQYzdHb3doM08ya3FYZUJJc3dnMUI0IiwgIldweFE0SFNvRXRjVG1DQ0tPZU",
    "RzbEJfZW11Y1lMejJvTzhvSE5yMWJFVlEiXX1d~WyJlSThaV205UW5LUHBOUGVOZ",
    "W5IZGhRIiwgIm1ldGhvZCIsICJwaXBwIl0~WyJHMDJOU3JRZmpGWFE3SW8wOXN5Y",
    "WpBIiwgImdpdmVuX25hbWUiLCAiTWF4Il0~WyJsa2x4RjVqTVlsR1RQVW92TU5Jd",
    "kNBIiwgImZhbWlseV9uYW1lIiwgIk1cdTAwZmNsbGVyIl0~WyJ5MXNWVTV3ZGZKY",
    "WhWZGd3UGdTN1JRIiwgImFkZHJlc3MiLCB7ImxvY2FsaXR5IjogIk1heHN0YWR0I",
    "iwgInBvc3RhbF9jb2RlIjogIjEyMzQ0IiwgImNvdW50cnkiOiAiREUiLCAic3RyZ",
    "WV0X2FkZHJlc3MiOiAiV2VpZGVuc3RyYVx1MDBkZmUgMjIifV0~",
);

/// Appendix A.2: the Processed SD-JWT Payload the RFC prints for it.
pub(super) const A2_PROCESSED: &str = r#"{
  "iss": "https://issuer.example.com",
  "iat": 1683000000,
  "exp": 1883000000,
  "verified_claims": {
    "verification": {
      "trust_framework": "de_aml",
      "evidence": [
        {
          "method": "pipp"
        }
      ],
      "time": "2012-04-23T18:25Z"
    },
    "claims": {
      "given_name": "Max",
      "family_name": "Müller",
      "address": {
        "locality": "Maxstadt",
        "postal_code": "12344",
        "country": "DE",
        "street_address": "Weidenstraße 22"
      }
    }
  }
}"#;
