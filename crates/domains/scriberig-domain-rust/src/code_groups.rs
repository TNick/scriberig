//! Creates deterministic source-evidence code groups.

use scriberig_core::{EntityId, Provenance};
use scriberig_domain_code::CodeGroup;

/// Creates one group boundary for each contiguous statement range.
/// # Parameters
/// - `statement_count`: Number of statements in the source block.
/// # Returns
/// Deterministic code groups with no generated descriptions.
#[must_use]
pub fn initial_groups(statement_count: usize) -> Vec<CodeGroup> {
    (0..statement_count)
        .map(|index| CodeGroup {
            entity: EntityId::from_bytes({
                let mut bytes = [0; 16];
                bytes.copy_from_slice(
                    &blake3::hash(format!("rust.code_group.{index}").as_bytes()).as_bytes()[..16],
                );
                bytes
            }),
            statements: Vec::new(),
            comment: None,
            description: None,
            provenance: None::<Provenance>,
        })
        .collect()
}
