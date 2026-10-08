//! [`LawfulBasis`] — the six grounds of GDPR Art. 6(1).

use serde::{Deserialize, Serialize};

/// The ground on which personal data is processed: one of the six points of
/// Art. 6(1) of Regulation (EU) 2016/679.
///
/// The set is closed because the Article closes it: processing is lawful *"only
/// if and to the extent that at least one of the following applies"*, and six
/// points follow.
///
/// Naming one is the controller's statement. Which point applies, and whether
/// its conditions are met, is not something this crate can know or check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum LawfulBasis {
    /// Point (a): the data subject has consented to the processing for one or
    /// more specific purposes. The only basis ESPR Art. 10(1)(e), toys Art.
    /// 20(10) and detergents Art. 22(h) admit for customer personal data.
    Consent,
    /// Point (b): necessary to perform a contract the data subject is party to,
    /// or to take steps at their request before entering into one.
    Contract,
    /// Point (c): necessary to comply with a legal obligation the controller is
    /// subject to. Art. 6(3) requires that obligation to be laid down by Union or
    /// Member State law.
    LegalObligation,
    /// Point (d): necessary to protect the vital interests of the data subject
    /// or of another natural person.
    VitalInterests,
    /// Point (e): necessary for a task carried out in the public interest or in
    /// the exercise of official authority vested in the controller.
    PublicTask,
    /// Point (f): necessary for the legitimate interests of the controller or a
    /// third party, unless the data subject's interests or rights override them.
    LegitimateInterests,
}
