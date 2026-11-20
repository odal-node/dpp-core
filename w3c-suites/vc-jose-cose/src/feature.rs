//! The suite's six features, and the four this harness implements.

/// A feature this harness implements. The suite skips a row whose feature the
/// implementation's manifest declares `false`, so the other two are never asked
/// for in a correct run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    /// A credential secured as a `vc+jwt` (VC-JOSE-COSE §3.1.1).
    CredentialJose,
    /// A presentation secured as a `vp+jwt` (VC-JOSE-COSE §3.1.2).
    PresentationJose,
    /// A credential secured as a `vc+sd-jwt` (VC-JOSE-COSE §3.2.1).
    CredentialSdJwt,
    /// A presentation secured as a `vp+sd-jwt` (VC-JOSE-COSE §3.2.2).
    PresentationSdJwt,
}

/// The features of `tests/test-mapping.js` that are not implemented: the COSE secured
/// forms (VC-JOSE-COSE §3.3).
const NOT_IMPLEMENTED: [&str; 2] = ["credential_cose", "presentation_cose"];

impl Feature {
    /// The feature the suite named, or why this harness will not answer for it.
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "credential_jose" => Ok(Self::CredentialJose),
            "presentation_jose" => Ok(Self::PresentationJose),
            "credential_sdjwt" => Ok(Self::CredentialSdJwt),
            "presentation_sdjwt" => Ok(Self::PresentationSdJwt),
            n if NOT_IMPLEMENTED.contains(&n) => Err(format!(
                "the feature {n} is not implemented, and its manifest entry must say false"
            )),
            n => Err(format!("unknown feature `{n}`")),
        }
    }

    /// Whether the feature secures a document as an SD-JWT, and so takes `--sd`.
    pub fn is_sd_jwt(self) -> bool {
        matches!(self, Self::CredentialSdJwt | Self::PresentationSdJwt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_jose_and_sd_jwt_features_are_implemented() {
        assert_eq!(
            Feature::parse("credential_jose"),
            Ok(Feature::CredentialJose)
        );
        assert_eq!(
            Feature::parse("presentation_jose"),
            Ok(Feature::PresentationJose)
        );
        assert_eq!(
            Feature::parse("credential_sdjwt"),
            Ok(Feature::CredentialSdJwt)
        );
        assert_eq!(
            Feature::parse("presentation_sdjwt"),
            Ok(Feature::PresentationSdJwt)
        );
    }

    #[test]
    fn only_the_sd_jwt_features_take_paths() {
        assert!(Feature::CredentialSdJwt.is_sd_jwt());
        assert!(Feature::PresentationSdJwt.is_sd_jwt());
        assert!(!Feature::CredentialJose.is_sd_jwt());
        assert!(!Feature::PresentationJose.is_sd_jwt());
    }

    #[test]
    fn the_cose_features_are_refused_by_name_and_not_answered() {
        for name in NOT_IMPLEMENTED {
            let refusal = Feature::parse(name).expect_err("not implemented");
            assert!(refusal.contains(name), "{refusal}");
            assert!(refusal.contains("not implemented"), "{refusal}");
        }
        assert!(Feature::parse("credential_jwt").is_err());
    }
}
