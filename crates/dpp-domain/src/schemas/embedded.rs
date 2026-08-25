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
