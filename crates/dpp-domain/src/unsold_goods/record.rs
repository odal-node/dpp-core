//! [`DiscardRecord`] — one discard of unsold consumer products.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::evidence::{EvidenceKind, EvidenceRequirement, derogation_evidence};
use super::exemption::EnterpriseSize;
use super::operation::WasteTreatmentOperation;
use super::receiver::ReceivingOperator;
use super::retention::{Disclosure, KeepUntil, keep_until};
use super::weight::DiscardedWeight;
use crate::identifier::CommodityCode;
use crate::unsold_goods::{DiscardReason, DiscardedQuantity};

/// One discard of unsold consumer products: what, when, how much, why, to what
/// treatment and to whom.
///
/// # Every discard is a record
///
/// ESPR Art. 24(1) has an operator disclose the number and weight discarded, the
/// reasons, and the proportion delivered to each treatment, and Art. 24(2) has it
/// demonstrate "the delivery and reception of the discarded products" on request.
/// None of that is limited to destruction under a derogation, so a record exists
/// for every discard. Donations are not discards (Impl. Reg. (EU) 2026/2,
/// recital 3: the disclosure "does not cover donated products") and are not
/// records.
///
/// The yearly disclosure is a computation over these: see
/// [`aggregate`](super::aggregate).
///
/// # Plain data
///
/// Nothing is refused. A record with no CN code, no weight or a derogation its
/// facts do not support is still a record of something that was discarded, and
/// hiding it would hide the discard. Such records are reported, by
/// `dpp_rules::lint::unsold_goods::lint_discard_record` and by the exclusions
/// [`aggregate`](super::aggregate) returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscardRecord {
    /// The day the products were discarded. It decides the financial year the
    /// discard is disclosed in, and the date the retention period runs from.
    pub discarded_on: NaiveDate,
    /// The CN codes of the product — Annex I note (d). More than one only where
    /// items sold together count as one unit (note (f)). Art. 3 fixes the depth
    /// the disclosure files them at; the record keeps the product's own code.
    pub cn_codes: Vec<CommodityCode>,
    /// What the products are, in the operator's words. It stays with the record, as
    /// part of what the operator can show. The disclosure's line takes the
    /// discloser's own wording for note (e) (see
    /// [`aggregate`](super::aggregate)): the records' descriptions joined would make
    /// a line as long as the year.
    pub description: String,
    /// Note (f): the number of units, and whether it is an estimate.
    pub units: DiscardedQuantity,
    /// Note (g): the weight, and whether it is an estimate.
    pub weight: DiscardedWeight,
    /// Whether packaging is included in [`Self::weight`].
    pub packaging_included: bool,
    /// The Del. Reg. (EU) 2026/296 Art. 2 derogation relied on, if any. `None`
    /// where no derogation applies: they except the destruction of Annex VII
    /// products from the Art. 25(1) prohibition, so a discard of anything else, or
    /// for the sole purpose of preparing for reuse, has none to cite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<DiscardReason>,
    /// The operator's own short label for why the products were discarded — note
    /// (h)'s "more detailed explanation", and, where [`Self::reason`] is `None`, the
    /// only statement of why.
    ///
    /// A label and not a sentence: records that share a derogation, categories,
    /// packaging answer and label share a line of the disclosure, so a label is
    /// chosen from a short list the operator keeps and not typed afresh for each
    /// record. A sentence per record would make a line per record and a table as
    /// long as the year. Surrounding whitespace is ignored, and the record schema
    /// bounds the length.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_label: Option<String>,
    /// The operation the products were delivered for.
    pub treatment: WasteTreatmentOperation,
    /// Who received them.
    pub receiver: ReceivingOperator,
}

impl DiscardRecord {
    /// What the acts have an operator of `size` be able to show for this discard.
    ///
    /// **Nothing**, where the operator is outside Arts. 24(1) and 25(1) on the day of
    /// the discard (see [`EnterpriseSize::is_exempt_on`]): it discloses nothing for
    /// the documentation to demonstrate, and is not bound by the ban a derogation
    /// excepts. Otherwise:
    ///
    /// - **Always**, documentation of the delivery and reception (ESPR Art. 24(2),
    ///   Impl. Reg. (EU) 2026/2 Art. 4).
    /// - **Where a treatment is stated**, a statement on the reception and
    ///   treatment (Art. 4). Where it is [`WasteTreatmentOperation::Unknown`] there
    ///   is nothing for a statement to confirm.
    /// - **Where a derogation is relied on**, its documentation under Del. Reg. (EU)
    ///   2026/296 Art. 3. The trigger is the claim, not Annex VII scope: Art. 24(2)
    ///   asks for the information to "demonstrate the applicability of a derogation"
    ///   where one is disclosed.
    ///
    /// The statement the operator gives the waste treatment operator (Art. 4) is a
    /// duty to provide, not to keep, and is not listed.
    #[must_use]
    pub fn required_evidence(&self, size: EnterpriseSize) -> Vec<EvidenceRequirement> {
        if size.is_exempt_on(self.discarded_on) {
            return Vec::new();
        }
        let mut required = vec![EvidenceRequirement {
            any_of: vec![EvidenceKind::DeliveryDocumentation],
            basis: "Regulation (EU) 2024/1781 Art. 24(2); Impl. Reg. (EU) 2026/2 Art. 4",
        }];
        if self.treatment != WasteTreatmentOperation::Unknown {
            required.push(EvidenceRequirement {
                any_of: vec![EvidenceKind::TreatmentStatement],
                basis: "Impl. Reg. (EU) 2026/2 Art. 4",
            });
        }
        if let Some(reason) = self.reason {
            required.extend(derogation_evidence(reason));
        }
        required
    }

    /// The requirements no held document meets.
    ///
    /// `held` is the kinds of the documents attached to the record. A gap is a
    /// document the act lists, not a breach: Del. Reg. (EU) 2026/296 Art. 3 excepts
    /// information the competent authority already has under another act, which
    /// only the caller can know.
    #[must_use]
    pub fn missing_evidence(
        &self,
        size: EnterpriseSize,
        held: &[EvidenceKind],
    ) -> Vec<EvidenceRequirement> {
        self.required_evidence(size)
            .into_iter()
            .filter(|requirement| !requirement.is_met_by(held))
            .collect()
    }

    /// Until when this record's documents must be kept — the later of five years
    /// after the discard and five years after the disclosure covering it.
    /// `disclosed_on` is the date that disclosure was made, if it has been; an
    /// operator that is exempt owes none, whatever it has made.
    ///
    /// Where the year is one the Regulation does not cover, which this crate cannot
    /// tell, use [`keep_until`] with [`Disclosure::NotOwed`].
    #[must_use]
    pub fn keep_until(&self, size: EnterpriseSize, disclosed_on: Option<NaiveDate>) -> KeepUntil {
        keep_until(
            self.discarded_on,
            Disclosure::of(size, self.discarded_on, disclosed_on),
        )
    }
}
