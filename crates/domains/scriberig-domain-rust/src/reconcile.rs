//! Reconciles parsed Rust syntax into unapplied shared migration proposals.

use std::collections::BTreeMap;

use scriberig_core::{EncodingVersion, EntityId, ObjectType, SnapshotId, revision_id};
use sr_change::{Operation, Provenance, SourceKind};

use crate::{
    RustFile, RustItem,
    change::RustChange,
    match_syntax::{CanonicalRustItem, SyntaxMatch, SyntaxMatchReport, match_items},
};

/// A Rust proposal that uses shared migration operations.
#[derive(Clone, Debug, PartialEq)]
pub struct RustMigrationProposal {
    /// Exact synchronization-base snapshot.
    pub base: SnapshotId,
    /// Shared semantic operations.
    pub operations: Vec<Operation>,
    /// Rust-specific local explanations.
    pub changes: Vec<RustChange>,
    /// Identity matching evidence.
    pub matches: SyntaxMatchReport,
    /// External projection provenance.
    pub provenance: Provenance,
}

/// Rust reconciliation result.
#[derive(Clone, Debug, PartialEq)]
pub enum RustReconciliation {
    /// Candidate has no semantic effect.
    NoSemanticChange,
    /// Candidate produced an unapplied proposal.
    MigrationProposal(Box<RustMigrationProposal>),
    /// Candidate contains an unsafe identity ambiguity.
    Ambiguous(SyntaxMatchReport),
}

/// Reconciles Rust syntax against one unchanged canonical base.
///
/// # Parameters
/// - `base_snapshot`: Exact synchronization-base snapshot.
/// - `base`: Canonical Rust items.
/// - `candidate`: Newly parsed Rust source.
/// - `projection`: Stable source/projection identifier.
///
/// # Returns
/// A no-op, explicit ambiguity, or unapplied shared migration proposal.
#[must_use]
pub fn reconcile(
    base_snapshot: SnapshotId,
    base: &[CanonicalRustItem],
    candidate: &RustFile,
    projection: impl Into<String>,
) -> RustReconciliation {
    reconcile_with_comments(base_snapshot, base, &[], candidate, projection)
}

/// Reconciles Rust syntax and source-authored comments against a complete base.
///
/// # Parameters
/// - `base_snapshot`: Exact synchronization-base snapshot.
/// - `base`: Canonical Rust items.
/// - `canonical_comments`: Comments retained by the canonical projection state.
/// - `candidate`: Newly parsed Rust source.
/// - `projection`: Stable source/projection identifier.
///
/// # Returns
/// A no-op, explicit ambiguity, or unapplied shared migration proposal.
#[must_use]
pub fn reconcile_with_comments(
    base_snapshot: SnapshotId,
    base: &[CanonicalRustItem],
    canonical_comments: &[crate::comments::RustComment],
    candidate: &RustFile,
    projection: impl Into<String>,
) -> RustReconciliation {
    let matches = match_items(base, candidate);
    if matches
        .matches
        .iter()
        .any(|item| matches!(item, SyntaxMatch::Ambiguous { .. }))
    {
        return RustReconciliation::Ambiguous(matches);
    }
    let mut operations = Vec::new();
    let mut changes = Vec::new();
    for item in &matches.matches {
        let SyntaxMatch::Matched {
            candidate: candidate_index,
            existing,
            ..
        } = item
        else {
            continue;
        };
        let Some(old) = base.iter().find(|value| value.id == *existing) else {
            continue;
        };
        let new_item = &candidate.items[*candidate_index];
        if old.item != *new_item {
            operations.push(Operation::ReplaceRevision {
                entity: *existing,
                revision: item_revision(new_item),
            });
            if let (RustItem::Function(old_function), RustItem::Function(new_function)) =
                (&old.item, new_item)
            {
                compare_statements(
                    *existing,
                    &old_function.body.statements,
                    &new_function.body.statements,
                    &mut changes,
                );
            }
        }
        if old.position != *candidate_index {
            changes.push(RustChange::MoveItem {
                entity: *existing,
                position: *candidate_index,
            });
        }
    }
    for item in &matches.matches {
        if let SyntaxMatch::New { candidate: index } = item {
            let entity = EntityId::new(new_entity_bytes(*index));
            operations.push(Operation::CreateEntity {
                entity,
                revision: item_revision(&candidate.items[*index]),
            });
        }
    }
    for entity in &matches.unmatched {
        operations.push(Operation::RemoveEntity { entity: *entity });
    }
    if candidate.comments != canonical_comments {
        changes.push(RustChange::CommentChange);
    }
    if operations.is_empty() && changes.is_empty() {
        return RustReconciliation::NoSemanticChange;
    }
    let mut metadata = BTreeMap::new();
    metadata.insert("projection".to_owned(), projection.into());
    RustReconciliation::MigrationProposal(Box::new(RustMigrationProposal {
        base: base_snapshot,
        operations,
        changes,
        matches,
        provenance: Provenance {
            source: SourceKind::ExternalProjection,
            actor: None,
            workspace: None,
            projection: None,
            validation: Vec::new(),
            parents: Vec::new(),
            metadata,
        },
    }))
}

/// Compares function statement sequences and records local changes.
///
/// # Parameters
/// - `function`: Function entity containing the statements.
/// - `base`: Existing statement sequence.
/// - `candidate`: Candidate statement sequence.
/// - `changes`: Destination local change list.
fn compare_statements(
    function: EntityId,
    base: &[crate::RustStatement],
    candidate: &[crate::RustStatement],
    changes: &mut Vec<RustChange>,
) {
    let shared = base.len().min(candidate.len());
    for index in 0..shared {
        if base[index] != candidate[index] {
            changes.push(RustChange::ReplaceStatement { function, index });
        }
    }
    for index in shared..candidate.len() {
        changes.push(RustChange::InsertStatement { function, index });
    }
    for index in shared..base.len() {
        changes.push(RustChange::RemoveStatement { function, index });
    }
}

/// Produces a revision identity from canonical item tokens.
///
/// # Parameters
/// - `item`: Rust syntax item.
///
/// # Returns
/// Revision identity for the item payload.
fn item_revision(item: &RustItem) -> scriberig_core::RevisionId {
    revision_id(
        EncodingVersion::V1,
        &ObjectType::new("rust.item").expect("static type"),
        format!("{item:?}").as_bytes(),
    )
    .expect("static canonical encoding version")
}

/// Produces a stable identity for one newly inserted candidate item.
///
/// # Parameters
/// - `index`: Candidate item index.
///
/// # Returns
/// A deterministic 128-bit entity identity.
fn new_entity_bytes(index: usize) -> [u8; 16] {
    let digest = blake3::hash(format!("rust.candidate.item:{index}").as_bytes());
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest.as_bytes()[..16]);
    bytes
}
