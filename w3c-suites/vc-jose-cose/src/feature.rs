//! The suite's six features, all of which this harness implements.

/// A feature this harness implements. The suite skips a row whose feature the
/// implementation's manifest declares `false`.
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
    /// A credential secured as a `vc+cose` (VC-JOSE-COSE §3.3.1).
    CredentialCose,
    /// A presentation secured as a `vp+cose` (VC-JOSE-COSE §3.3.2).
    PresentationCose,
}

impl Feature {
    /// The feature the suite named, or why this harness will not answer for it: it is not
    /// one of the suite's six.
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "credential_jose" => Ok(Self::CredentialJose),
            "presentation_jose" => Ok(Self::PresentationJose),
            "credential_sdjwt" => Ok(Self::CredentialSdJwt),
            "presentation_sdjwt" => Ok(Self::PresentationSdJwt),
            "credential_cose" => Ok(Self::CredentialCose),
            "presentation_cose" => Ok(Self::PresentationCose),
            n => Err(format!("unknown feature `{n}`")),
        }
    }

    /// Whether the feature secures a document as an SD-JWT, and so takes `--sd`.
    pub fn is_sd_jwt(self) -> bool {
        matches!(self, Self::CredentialSdJwt | Self::PresentationSdJwt)
    }

    /// Whether the feature secures a document with COSE, whose input and output files are
    /// base64 text of the `COSE_Sign1` message.
    pub fn is_cose(self) -> bool {
        matches!(self, Self::CredentialCose | Self::PresentationCose)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_six_features_are_implemented() {
        for (name, feature) in [
            ("credential_jose", Feature::CredentialJose),
            ("presentation_jose", Feature::PresentationJose),
            ("credential_sdjwt", Feature::CredentialSdJwt),
            ("presentation_sdjwt", Feature::PresentationSdJwt),
            ("credential_cose", Feature::CredentialCose),
            ("presentation_cose", Feature::PresentationCose),
        ] {
            assert_eq!(Feature::parse(name), Ok(feature), "{name}");
        }
    }

    #[test]
    fn only_the_sd_jwt_features_take_paths() {
        assert!(Feature::CredentialSdJwt.is_sd_jwt());
        assert!(Feature::PresentationSdJwt.is_sd_jwt());
        assert!(!Feature::CredentialJose.is_sd_jwt());
        assert!(!Feature::PresentationJose.is_sd_jwt());
        assert!(!Feature::CredentialCose.is_sd_jwt());
        assert!(!Feature::PresentationCose.is_sd_jwt());
    }

    #[test]
    fn only_the_cose_features_read_and_write_base64() {
        assert!(Feature::CredentialCose.is_cose());
        assert!(Feature::PresentationCose.is_cose());
        assert!(!Feature::CredentialJose.is_cose());
        assert!(!Feature::CredentialSdJwt.is_cose());
    }

    #[test]
    fn a_feature_the_suite_does_not_have_is_refused_by_name() {
        let refusal = Feature::parse("credential_jwt").expect_err("unknown");
        assert!(refusal.contains("credential_jwt"), "{refusal}");
        assert!(Feature::parse("").is_err());
    }
}
