use semver::Version;

use super::{SchemaEntry, SchemaOrigin};

struct EmbeddedSchema {
    product_group: &'static str,
    version: &'static str,
    json: &'static str,
}

const EMBEDDED: &[EmbeddedSchema] = &[
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
    EmbeddedSchema {
        product_group: "unsold-goods",
        version: "1.0.0",
        json: include_str!("../../schemas/unsold-goods/v1.0.0.json"),
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
