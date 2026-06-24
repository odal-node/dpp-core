//! GS1 Application Identifier (AI) type table for Digital Link URI paths.

/// Role of a GS1 Application Identifier within a Digital Link URI path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiRole {
    PrimaryKey,
    Qualifier,
    DataAttribute,
}

pub struct AiDescriptor {
    pub code: &'static str,
    pub role: AiRole,
    pub title: &'static str,
    pub max_len: usize,
    /// Canonical qualifier order within the path; `None` for PrimaryKey.
    pub qualifier_order: Option<u8>,
}

/// Static table of recognised GS1 Application Identifiers for DL URI paths.
pub const AI_TABLE: &[AiDescriptor] = &[
    AiDescriptor {
        code: "01",
        role: AiRole::PrimaryKey,
        title: "GTIN",
        max_len: 14,
        qualifier_order: None,
    },
    AiDescriptor {
        code: "22",
        role: AiRole::Qualifier,
        title: "Consumer Product Variant",
        max_len: 20,
        qualifier_order: Some(1),
    },
    AiDescriptor {
        code: "10",
        role: AiRole::Qualifier,
        title: "Batch/Lot Number",
        max_len: 20,
        qualifier_order: Some(2),
    },
    AiDescriptor {
        code: "21",
        role: AiRole::Qualifier,
        title: "Serial Number",
        max_len: 20,
        qualifier_order: Some(3),
    },
    AiDescriptor {
        code: "235",
        role: AiRole::Qualifier,
        title: "Third-Party Controlled Serial",
        max_len: 28,
        qualifier_order: Some(4),
    },
];

/// Look up a descriptor by AI code, returning `None` for unknown codes.
pub fn ai_descriptor(code: &str) -> Option<&'static AiDescriptor> {
    AI_TABLE.iter().find(|d| d.code == code)
}
