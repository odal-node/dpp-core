use semver::Version;

use super::{SchemaEntry, SchemaOrigin};

struct EmbeddedSchema {
    sector: &'static str,
    version: &'static str,
    json: &'static str,
}

const EMBEDDED: &[EmbeddedSchema] = &[
    EmbeddedSchema {
        sector: "battery",
        version: "1.0.0",
        json: include_str!("../../schemas/battery/v1.0.0.json"),
    },
    EmbeddedSchema {
        sector: "battery",
        version: "2.0.0",
        json: include_str!("../../schemas/battery/v2.0.0.json"),
    },
    EmbeddedSchema {
        sector: "textile",
        version: "1.0.0",
        json: include_str!("../../schemas/textile/v1.0.0.json"),
    },
    EmbeddedSchema {
        sector: "textile",
        version: "1.1.0",
        json: include_str!("../../schemas/textile/v1.1.0.json"),
    },
    EmbeddedSchema {
        sector: "textile",
        version: "1.2.0",
        json: include_str!("../../schemas/textile/v1.2.0.json"),
    },
    EmbeddedSchema {
        sector: "unsold-goods",
        version: "1.0.0",
        json: include_str!("../../schemas/unsold-goods/v1.0.0.json"),
    },
    EmbeddedSchema {
        sector: "steel",
        version: "1.0.0",
        json: include_str!("../../schemas/steel/v1.0.0.json"),
    },
    EmbeddedSchema {
        sector: "steel",
        version: "1.1.0",
        json: include_str!("../../schemas/steel/v1.1.0.json"),
    },
    EmbeddedSchema {
        sector: "electronics",
        version: "1.0.0",
        json: include_str!("../../schemas/electronics/v1.0.0.json"),
    },
    EmbeddedSchema {
        sector: "electronics",
        version: "1.1.0",
        json: include_str!("../../schemas/electronics/v1.1.0.json"),
    },
    EmbeddedSchema {
        sector: "construction",
        version: "1.0.0",
        json: include_str!("../../schemas/construction/v1.0.0.json"),
    },
    EmbeddedSchema {
        sector: "construction",
        version: "1.1.0",
        json: include_str!("../../schemas/construction/v1.1.0.json"),
    },
    EmbeddedSchema {
        sector: "tyre",
        version: "1.0.0",
        json: include_str!("../../schemas/tyre/v1.0.0.json"),
    },
    EmbeddedSchema {
        sector: "toy",
        version: "1.0.0",
        json: include_str!("../../schemas/toy/v1.0.0.json"),
    },
    EmbeddedSchema {
        sector: "toy",
        version: "1.1.0",
        json: include_str!("../../schemas/toy/v1.1.0.json"),
    },
    EmbeddedSchema {
        sector: "aluminium",
        version: "1.0.0",
        json: include_str!("../../schemas/aluminium/v1.0.0.json"),
    },
    EmbeddedSchema {
        sector: "aluminium",
        version: "1.1.0",
        json: include_str!("../../schemas/aluminium/v1.1.0.json"),
    },
    EmbeddedSchema {
        sector: "furniture",
        version: "1.0.0",
        json: include_str!("../../schemas/furniture/v1.0.0.json"),
    },
    EmbeddedSchema {
        sector: "furniture",
        version: "1.1.0",
        json: include_str!("../../schemas/furniture/v1.1.0.json"),
    },
    EmbeddedSchema {
        sector: "detergent",
        version: "1.0.0",
        json: include_str!("../../schemas/detergent/v1.0.0.json"),
    },
    EmbeddedSchema {
        sector: "detergent",
        version: "1.1.0",
        json: include_str!("../../schemas/detergent/v1.1.0.json"),
    },
];

pub(super) fn initial_entries() -> Vec<SchemaEntry> {
    EMBEDDED
        .iter()
        .map(|e| SchemaEntry {
            sector: e.sector.to_owned(),
            version: e
                .version
                .parse::<Version>()
                .expect("embedded schema version is valid semver"),
            json: e.json.to_owned(),
            origin: SchemaOrigin::Embedded,
        })
        .collect()
}
