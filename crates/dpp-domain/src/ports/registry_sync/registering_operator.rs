//! [`RegisteringOperator`] — the operator details a passport does not carry.

/// The registering operator's own details, which the passport does not carry.
///
/// A struct rather than loose arguments because `legal_name` and `country` are
/// both plain strings: passed positionally they can be swapped without the
/// compiler noticing, and the result is a registration filed under the wrong
/// legal entity.
#[derive(Debug, Clone, Copy)]
pub struct RegisteringOperator<'a> {
    /// Legal name of the economic operator (`OperatorConfig.legal_name`).
    pub legal_name: &'a str,
    /// ISO 3166-1 alpha-2 country of registration (`OperatorConfig.country`).
    pub country: &'a str,
    /// Scheme of the operator's primary identifier — the `scheme` column beside
    /// the value the passport was stamped with. Belongs here rather than on the
    /// passport for the same reason the other two do: it is a fact about the
    /// operator, not about the product.
    pub identifier_scheme: &'a str,
}
