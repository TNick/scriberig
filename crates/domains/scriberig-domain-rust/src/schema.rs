//! Defines the Rust syntax schema specialization.

/// Rust-specific source entity types supported by the adapter.
pub const RUST_ENTITY_TYPES: &[&str] = &[
    "rust.file",
    "rust.function",
    "rust.parameter",
    "rust.struct",
    "rust.enum",
    "rust.impl",
    "rust.macro",
    "rust.statement",
    "rust.expression",
    "rust.comment",
];

/// Rust syntax relations.
pub const RUST_RELATIONS: &[&str] = &[
    "rust.contains",
    "rust.parameters",
    "rust.attributes",
    "rust.comments",
    "rust.groups",
];
