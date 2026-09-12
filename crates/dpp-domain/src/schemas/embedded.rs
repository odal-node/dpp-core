//! The compile-time schema table: every product-group schema `include_str!`d into
//! the binary, so a published crate carries its own schemas.
//!
//! # Known errata in published schema versions
//!
//! **A published schema version is never modified.** A passport validated
//! against `v1` stays valid against `v1` indefinitely, and rewriting one would
//! break that. So where a *citation* in a published version is wrong, the
//! version keeps it and the correction is recorded here.
//!
//! This list lives beside the `include_str!` table, and not in a repository
//! document, deliberately: it has to reach whoever is actually reading the
//! affected version, and that reader may have vendored this crate from
//! crates.io with no access to anything else.
//!
//! ⚠️ Every entry below is a **field description**. None is a constant, a
//! threshold or an enumerated category, and nothing branches on any of them — so
//! no validation behaviour changes, in any version, in either direction.
//!
//! ## `battery/v2.0.0` — `disassemblyInstructionsUrl` cites "Annex XIII §6"
//!
//! The description reads *"(Annex XIII §6)"*. **Annex XIII of Regulation (EU)
//! 2023/1542 has four numbered sections and no §5 or §6.** Dismantling
//! information is **point 2(c)** — *"dismantling information, including at
//! least:"*, under the heading for information accessible only to persons with a
//! legitimate interest and the Commission.
//!
//! Corrected from `v2.1.0` onward, which reads "(Annex XIII point 2(c))".
//!
//! ## `battery/v2.6.0` — `markingInformation` and `hazardSymbol` cite Art. 13(4)
//!
//! Annex XIII point 1(q) was amended by **corrigendum C4, OJ L, 10.4.2026**,
//! from *"the marking requirements laid down in Article 13(3) and (4)"* to
//! *"…in **Article 13(4) and (5)**"*.
//!
//! The change is not cosmetic: Art. 13 splits at the verb, with paragraphs 1–3
//! saying batteries *"shall bear a label"* and paragraphs 4 and 5 saying they
//! *"shall be marked"*. The corrigendum moves point 1(q) onto the two **marking**
//! paragraphs — the separate-collection symbol and the Cd/Pb chemical symbol.
//!
//! Two consequences for this version. `markingInformation` cites only Art. 13(4)
//! and is one limb short of "13(4) and (5)". `hazardSymbol` states that it is
//! *"not itself an Annex XIII point 1(q) item"*; under the corrigendum **it is
//! one**. Both descriptions were correct against the text as it stood when they
//! were written, and the act moved underneath them.
//!
//! ## `textile/v1.1.0` and `v1.2.0` — `allergens` misdescribes REACH entry 72
//!
//! The description reads *"Contact allergens regulated under Regulation (EC) No
//! 1907/2006 (REACH) Annex XVII entry 72 (e.g. certain disperse dyes, chromium
//! VI, nickel in accessories)."* Two of those three examples hold; the framing
//! around them does not.
//!
//! **Nickel is not in entry 72.** Nickel release from articles in prolonged skin
//! contact is **Annex XVII entry 27**, a separate and much older restriction
//! with its own migration-limit regime. It has had its own entry since long
//! before entry 72 existed, so this is structural rather than a question of
//! which consolidation is read.
//!
//! **"Contact allergens" mischaracterises the entry.** Entry 72's Appendix 12 is
//! predominantly a **CMR and PAH** restriction — cadmium, arsenic, lead, benzene
//! and seven polycyclic aromatic hydrocarbons are carcinogens, mutagens or
//! reprotoxicants rather than skin sensitisers. Chromium VI and C.I. Disperse
//! Blue 1 are sensitisers; most of the list is not. So the description asks an
//! operator to declare a narrower and differently-aimed class of substance than
//! the entry covers.
//!
//! What the entry restricts, correctly stated: substances listed in Appendix 12,
//! in clothing, skin-contact textiles and footwear, at or above the concentration
//! given for each substance, measured in homogeneous material. The entry number,
//! the scope and the homogeneous-material basis were all right.
//!
//! ⚠️ Read from Commission Regulation (EU) 2018/1513, which introduced entry 72
//! and Appendix 12. **Appendix 12's current membership was not re-checked**
//! against the latest consolidation, so it may have gained substances since
//! 2018. That does not touch either finding above.

use semver::Version;

use super::{SchemaEntry, SchemaOrigin};

pub(crate) struct EmbeddedSchema {
    pub(crate) product_group: &'static str,
    pub(crate) version: &'static str,
    pub(crate) json: &'static str,
}

pub(crate) const EMBEDDED: &[EmbeddedSchema] = &[
    EmbeddedSchema {
        product_group: "battery",
        version: "1.0.0",
        json: include_str!("../../schemas/battery/v1.0.0.json"),
    },
    EmbeddedSchema {
        product_group: "battery",
        version: "2.0.0",
        json: include_str!("../../schemas/battery/v2.0.0.json"),
    },
    EmbeddedSchema {
        product_group: "battery",
        version: "2.1.0",
        json: include_str!("../../schemas/battery/v2.1.0.json"),
    },
    EmbeddedSchema {
        product_group: "battery",
        version: "2.2.0",
        json: include_str!("../../schemas/battery/v2.2.0.json"),
    },
    EmbeddedSchema {
        product_group: "battery",
        version: "2.3.0",
        json: include_str!("../../schemas/battery/v2.3.0.json"),
    },
    EmbeddedSchema {
        product_group: "battery",
        version: "2.4.0",
        json: include_str!("../../schemas/battery/v2.4.0.json"),
    },
    EmbeddedSchema {
        product_group: "battery",
        version: "2.5.0",
        json: include_str!("../../schemas/battery/v2.5.0.json"),
    },
    EmbeddedSchema {
        product_group: "battery",
        version: "2.6.0",
        json: include_str!("../../schemas/battery/v2.6.0.json"),
    },
    EmbeddedSchema {
        product_group: "textile",
        version: "1.0.0",
        json: include_str!("../../schemas/textile/v1.0.0.json"),
    },
    EmbeddedSchema {
        product_group: "textile",
        version: "1.1.0",
        json: include_str!("../../schemas/textile/v1.1.0.json"),
    },
    EmbeddedSchema {
        product_group: "textile",
        version: "1.2.0",
        json: include_str!("../../schemas/textile/v1.2.0.json"),
    },
    // No v1.0.0. It predated Impl. Reg. (EU) 2026/2 and nothing can carry a
    // document forward from it: a financial year is not derivable from a
    // quarter, a CN code is not derivable from the word "apparel", a six-way
    // treatment split is not derivable from one destination, and its reason
    // list has no member in common with the Art. 2 derogations. A lens would
    // have to invent every one of those, so the version was removed rather than
    // migrated. Safe only because nothing has ever been stored under it.
    EmbeddedSchema {
        product_group: "unsold-goods",
        version: "2.0.0",
        json: include_str!("../../schemas/unsold-goods/v2.0.0.json"),
    },
    EmbeddedSchema {
        product_group: "steel",
        version: "1.0.0",
        json: include_str!("../../schemas/steel/v1.0.0.json"),
    },
    EmbeddedSchema {
        product_group: "steel",
        version: "1.1.0",
        json: include_str!("../../schemas/steel/v1.1.0.json"),
    },
    EmbeddedSchema {
        product_group: "electronics",
        version: "1.0.0",
        json: include_str!("../../schemas/electronics/v1.0.0.json"),
    },
    EmbeddedSchema {
        product_group: "electronics",
        version: "1.1.0",
        json: include_str!("../../schemas/electronics/v1.1.0.json"),
    },
    EmbeddedSchema {
        product_group: "electronics",
        version: "1.2.0",
        json: include_str!("../../schemas/electronics/v1.2.0.json"),
    },
    EmbeddedSchema {
        product_group: "construction",
        version: "1.0.0",
        json: include_str!("../../schemas/construction/v1.0.0.json"),
    },
    EmbeddedSchema {
        product_group: "construction",
        version: "1.1.0",
        json: include_str!("../../schemas/construction/v1.1.0.json"),
    },
    EmbeddedSchema {
        product_group: "tyre",
        version: "1.0.0",
        json: include_str!("../../schemas/tyre/v1.0.0.json"),
    },
    EmbeddedSchema {
        product_group: "toy",
        version: "1.0.0",
        json: include_str!("../../schemas/toy/v1.0.0.json"),
    },
    EmbeddedSchema {
        product_group: "toy",
        version: "1.1.0",
        json: include_str!("../../schemas/toy/v1.1.0.json"),
    },
    EmbeddedSchema {
        product_group: "aluminium",
        version: "1.0.0",
        json: include_str!("../../schemas/aluminium/v1.0.0.json"),
    },
    EmbeddedSchema {
        product_group: "aluminium",
        version: "1.1.0",
        json: include_str!("../../schemas/aluminium/v1.1.0.json"),
    },
    EmbeddedSchema {
        product_group: "furniture",
        version: "1.0.0",
        json: include_str!("../../schemas/furniture/v1.0.0.json"),
    },
    EmbeddedSchema {
        product_group: "furniture",
        version: "1.1.0",
        json: include_str!("../../schemas/furniture/v1.1.0.json"),
    },
    EmbeddedSchema {
        product_group: "furniture",
        version: "1.2.0",
        json: include_str!("../../schemas/furniture/v1.2.0.json"),
    },
    EmbeddedSchema {
        product_group: "mattress",
        version: "1.0.0",
        json: include_str!("../../schemas/mattress/v1.0.0.json"),
    },
    EmbeddedSchema {
        product_group: "detergent",
        version: "1.0.0",
        json: include_str!("../../schemas/detergent/v1.0.0.json"),
    },
    EmbeddedSchema {
        product_group: "detergent",
        version: "1.1.0",
        json: include_str!("../../schemas/detergent/v1.1.0.json"),
    },
];

pub(super) fn initial_entries() -> Vec<SchemaEntry> {
    EMBEDDED
        .iter()
        .map(|e| SchemaEntry {
            product_group: e.product_group.to_owned(),
            version: e
                .version
                .parse::<Version>()
                .expect("embedded schema version is valid semver"),
            json: e.json.to_owned(),
            origin: SchemaOrigin::Embedded,
        })
        .collect()
}
