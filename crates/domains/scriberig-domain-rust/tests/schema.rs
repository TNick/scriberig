//! Tests Rust schema specialization.
use scriberig_domain_rust::{RUST_ENTITY_TYPES, RUST_RELATIONS};

/// Ensures Rust schema names specialize shared code concepts.
#[test]
fn schema_contains_rust_constructs() {
    assert!(RUST_ENTITY_TYPES.contains(&"rust.function"));
    assert!(RUST_ENTITY_TYPES.contains(&"rust.macro"));
    assert!(RUST_RELATIONS.contains(&"rust.comments"));
}
