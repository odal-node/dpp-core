//! [`UnmodelledPayload`] — the data of a product group this build has no typed
//! variant for.

use crate::identifier::ProductIdentifier;
use crate::product_group::ProductGroupPayload;

use super::common::SvhcSubstance;

/// The payload of a product group this build has no typed variant for: the
/// wire tag, the whole object as received, and the identifier it carries.
///
/// # Why the fields are private
///
/// Three facts about this value have to hold together, and public fields let
/// any caller break each of them:
///
/// - `data` is a JSON **object** carrying its own `productGroup` key. The
///   serialiser only stamps the tag onto an object, so an array or scalar
///   would serialise untagged — and slip past the fail-closed reduction a
///   passport view applies to a product group it has no policy for.
/// - the tag names a group this build does **not** type. `Other("battery")`
///   would be a second representation of battery that compares unequal to the
///   first and misses every typed match.
/// - the identifier is the one `data` carries. It is read once, when the value
///   is built, and a caller able to replace `data` afterwards would leave a
///   carrier or a registration naming an identifier the signed passport does
///   not contain.
///
/// So the only ways to build one are deserialising a `ProductGroupData` and
/// [`ProductGroupData::other`](super::ProductGroupData::other), which enforce
/// the first two, and `data` is read-only, which keeps the third.
#[derive(Debug, Clone, PartialEq)]
pub struct UnmodelledPayload {
    product_group: String,
    data: serde_json::Value,
    identifier: Option<ProductIdentifier>,
}

impl UnmodelledPayload {
    /// Build from a tag and an object the caller has already checked.
    ///
    /// # The identifier is read leniently
    ///
    /// `productIdentifier` is parsed through the same EN 18219 clause 5
    /// deserialiser every typed payload uses. A value that fails it leaves the
    /// payload with no identifier rather than failing the whole payload: an
    /// untyped group must keep round-tripping whatever it carries, and a
    /// passport fetched from another operator cannot be rewritten to fix it.
    /// Nothing is let through by it: a malformed identifier is refused
    /// wherever one is required, exactly as an absent one is.
    pub(super) fn new(product_group: String, data: serde_json::Value) -> Self {
        let identifier = data
            .get("productIdentifier")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok());
        Self {
            product_group,
            data,
            identifier,
        }
    }

    /// The wire tag exactly as received.
    #[must_use]
    pub fn product_group(&self) -> &str {
        &self.product_group
    }

    /// The full object, including its `productGroup` key.
    #[must_use]
    pub fn data(&self) -> &serde_json::Value {
        &self.data
    }
}

/// An untyped payload answers the one question every product group asks the
/// same way — which product it identifies — and nothing else.
///
/// The identifier's key and shape are fixed by EN 18219 clause 5 rather than
/// by any one act, which is why it can be read without a typed variant. The
/// other three are named differently by each act, or not defined at all, so
/// no key in an untyped object can be taken to mean them.
impl ProductGroupPayload for UnmodelledPayload {
    fn product_identifier(&self) -> Option<&ProductIdentifier> {
        self.identifier.as_ref()
    }

    fn model_identifier(&self) -> Option<&str> {
        None
    }

    fn svhc_substances(&self) -> Option<&[SvhcSubstance]> {
        None
    }

    fn product_category(&self) -> Option<&str> {
        None
    }
}
