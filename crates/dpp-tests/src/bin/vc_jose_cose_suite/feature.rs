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
mod tests;
