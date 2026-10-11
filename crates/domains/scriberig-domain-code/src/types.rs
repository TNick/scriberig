//! Defines the shared language-independent code vocabulary.
/// Stable shared code entity type identifiers.
pub const ENTITY_TYPES: &[&str] = &[
    "code.project",
    "code.module",
    "code.type",
    "code.function",
    "code.parameter",
    "code.code_group",
    "code.statement",
    "code.expression",
    "code.comment",
    "code.symbol",
    "code.reference",
];
/// Stable shared code relation identifiers.
pub const RELATIONS: &[&str] = &[
    "code.contains",
    "code.parameters",
    "code.code_groups",
    "code.attached_to",
    "code.targets",
];
