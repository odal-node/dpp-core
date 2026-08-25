//! [`VocabularyRegister`] — every upstream vocabulary we have recorded, and the
//! gate that decides whether an identifier may be emitted.

use crate::namespace::is_own;
use crate::vocabulary::Vocabulary;

use super::verdict::Verdict;

struct Embedded {
    key: &'static str,
    json: &'static str,
}

/// One record per authority. Adding a vocabulary is a single entry plus a file
/// in `vocabularies/`, exactly as adding a product group is in `dpp-domain`.
const EMBEDDED: &[Embedded] = &[
    Embedded {
        key: "gs1",
        json: include_str!("../../vocabularies/gs1.json"),
    },
    Embedded {
        key: "semic",
        json: include_str!("../../vocabularies/semic.json"),
    },
    Embedded {
        key: "schema-org",
        json: include_str!("../../vocabularies/schema-org.json"),
    },
    Embedded {
        key: "untp",
        json: include_str!("../../vocabularies/untp.json"),
    },
    Embedded {
        key: "dpp-keystone",
        json: include_str!("../../vocabularies/dpp-keystone.json"),
    },
    Embedded {
        key: "cirpass2",
        json: include_str!("../../vocabularies/cirpass2.json"),
    },
    Embedded {
        key: "gs1-rail",
        json: include_str!("../../vocabularies/gs1-rail.json"),
    },
    Embedded {
        key: "jtc24",
        json: include_str!("../../vocabularies/jtc24.json"),
    },
    Embedded {
        key: "batterypass-samm",
        json: include_str!("../../vocabularies/batterypass-samm.json"),
    },
    Embedded {
        key: "catena-x-battery-pass",
        json: include_str!("../../vocabularies/catena-x-battery-pass.json"),
    },
    Embedded {
        key: "batterypass-ready",
        json: include_str!("../../vocabularies/batterypass-ready.json"),
    },
    Embedded {
        key: "ec-battery-guidance",
        json: include_str!("../../vocabularies/ec-battery-guidance.json"),
    },
    Embedded {
        key: "idta",
        json: include_str!("../../vocabularies/idta.json"),
    },
    Embedded {
        key: "eclass",
        json: include_str!("../../vocabularies/eclass.json"),
    },
];

/// The upstream vocabularies this project has recorded.
///
/// ```
/// use dpp_vocab::VocabularyRegister;
///
/// let register = VocabularyRegister::new();
/// assert!(register.get("gs1").is_some());
///
/// // Our own identifiers are always permitted.
/// assert!(register.verdict("urn:odal-node:aas:property:product-name:1.0").is_permitted());
///
/// // GS1's own definition of `gtin` was read on 2026-08-11 and matches our
/// // `Gtin` type exactly, so its terms are now permitted.
/// assert!(register.verdict("https://ref.gs1.org/voc/gtin").is_permitted());
///
/// // Most third-party identifiers still are not — nobody has read them yet.
/// assert!(!register.verdict("https://schema.org/dateModified").is_permitted());
/// ```
#[derive(Debug, Clone)]
pub struct VocabularyRegister {
    entries: Vec<Vocabulary>,
}

impl VocabularyRegister {
    /// Load the register from the embedded records.
    ///
    /// # Panics
    ///
    /// If an embedded record is not valid JSON for a [`Vocabulary`], or if its
    /// `key` disagrees with its filename. Both are compile-time-authored data,
    /// so a failure here is a build defect rather than a runtime condition —
    /// the same contract `ProductGroupCatalog` makes about product group manifests.
    #[must_use]
    pub fn new() -> Self {
        let entries = EMBEDDED
            .iter()
            .map(|e| {
                let vocabulary: Vocabulary = serde_json::from_str(e.json).unwrap_or_else(|err| {
                    panic!("embedded vocabulary record '{}' is invalid: {err}", e.key)
                });
                assert_eq!(
                    vocabulary.key, e.key,
                    "vocabulary key '{}' does not match its file key '{}'",
                    vocabulary.key, e.key
                );
                vocabulary
            })
            .collect();
        Self { entries }
    }

    /// Look up a vocabulary by its key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Vocabulary> {
        self.entries.iter().find(|v| v.key == key)
    }

    /// Every recorded vocabulary.
    #[must_use]
    pub fn all(&self) -> &[Vocabulary] {
        &self.entries
    }

    /// The vocabulary whose namespace `iri` sits under, if any.
    ///
    /// Longest-prefix wins, so a vocabulary nested inside another's namespace
    /// resolves to the more specific one rather than to whichever was loaded
    /// first.
    #[must_use]
    pub fn vocabulary_for(&self, iri: &str) -> Option<&Vocabulary> {
        self.entries
            .iter()
            .filter(|v| v.contains(iri))
            .max_by_key(|v| v.namespace_iri.as_deref().map_or(0, str::len))
    }

    /// Whether `iri` may be emitted in a serialisation, and why.
    ///
    /// This is the gate. It answers at vocabulary granularity: a verified
    /// vocabulary makes its terms *eligible*, and per-term verification is a
    /// second question that a caller records separately. Today no vocabulary is
    /// verified, so every third-party identifier is refused — which is the
    /// accurate state of this project's knowledge, not a placeholder.
    #[must_use]
    pub fn verdict(&self, iri: &str) -> Verdict {
        if is_own(iri) {
            return Verdict::Own;
        }
        let Some(vocabulary) = self.vocabulary_for(iri) else {
            return Verdict::Unknown;
        };
        if !vocabulary.layer.may_be_adopted_as_prefix() {
            return Verdict::BridgeNotSpeakable {
                vocabulary: vocabulary.key.clone(),
            };
        }
        if !vocabulary.status.permits_emission() {
            return Verdict::NotVerified {
                vocabulary: vocabulary.key.clone(),
                status: vocabulary.status,
            };
        }
        Verdict::Permitted {
            vocabulary: vocabulary.key.clone(),
        }
    }
}

impl Default for VocabularyRegister {
    fn default() -> Self {
        Self::new()
    }
}
