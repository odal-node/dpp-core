//! The suite's six features, and the two this harness implements.

/// A feature this harness implements. The suite skips a row whose feature the
/// implementation's manifest declares `false`, so the other four are never asked
/// for in a correct run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    /// A credential secured as a `vc+jwt` (VC-JOSE-COSE §3.1.1).
    CredentialJose,
    /// A presentation secured as a `vp+jwt` (VC-JOSE-COSE §3.1.2).
    PresentationJose,
}

/// The features of `tests/test-mapping.js` that are not implemented: the SD-JWT and
/// COSE secured forms (VC-JOSE-COSE §3.2 and §3.3).
const NOT_IMPLEMENTED: [&str; 4] = [
    "credential_sdjwt",
    "presentation_sdjwt",
    "credential_cose",
    "presentation_cose",
];

impl Feature {
    /// The feature the suite named, or why this harness will not answer for it.
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "credential_jose" => Ok(Self::CredentialJose),
            "presentation_jose" => Ok(Self::PresentationJose),
            n if NOT_IMPLEMENTED.contains(&n) => Err(format!(
                "the feature {n} is not implemented, and its manifest entry must say false"
            )),
            n => Err(format!("unknown feature `{n}`")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_jose_features_are_implemented() {
        assert_eq!(
            Feature::parse("credential_jose"),
            Ok(Feature::CredentialJose)
        );
        assert_eq!(
            Feature::parse("presentation_jose"),
            Ok(Feature::PresentationJose)
        );
    }

    #[test]
    fn the_other_four_are_refused_by_name_and_not_answered() {
        for name in NOT_IMPLEMENTED {
            let refusal = Feature::parse(name).expect_err("not implemented");
            assert!(refusal.contains(name), "{refusal}");
            assert!(refusal.contains("not implemented"), "{refusal}");
        }
        assert!(Feature::parse("credential_jwt").is_err());
    }
}
