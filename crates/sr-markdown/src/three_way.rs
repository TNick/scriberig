//! Merges Markdown semantic states using an explicit base/current/external triplet.

use crate::{MarkdownBlock, MarkdownDocument, MarkdownStyle, render_document};
use scriberig_core::SnapshotId;

/// The semantic operation involved in a Markdown three-way conflict.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkdownConflictKind {
    /// Both sides changed one base block differently.
    SameBlockChanged,
    /// One side removed a block while the other changed it.
    DeleteModify,
    /// Collection ordering or section structure cannot be reconciled safely.
    Structural,
    /// The external candidate was already ambiguous before merging.
    Ambiguous,
}

/// Structured Markdown conflict retaining all three semantic values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownConflict {
    /// Conflict operation kind.
    pub kind: MarkdownConflictKind,
    /// Top-level semantic path where the conflict occurred.
    pub path: Vec<usize>,
    /// Base block, when it exists.
    pub base: Option<MarkdownBlock>,
    /// Current canonical block, when it exists.
    pub current: Option<MarkdownBlock>,
    /// External candidate block, when it exists.
    pub external: Option<MarkdownBlock>,
}

/// Result of explicit three-way Markdown reconciliation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ThreeWayOutcome {
    /// The external representation changed only formatting.
    FormattingOnly {
        /// Current canonical semantic document to rerender.
        document: MarkdownDocument,
    },
    /// Semantic changes merged without conflict.
    Merged {
        /// Combined semantic document.
        document: MarkdownDocument,
    },
    /// Same semantic result was produced independently by both sides.
    Converged {
        /// Shared resulting document.
        document: MarkdownDocument,
    },
    /// One or more semantic conflicts require resolution.
    Conflict {
        /// Structured conflict details.
        conflicts: Vec<MarkdownConflict>,
    },
    /// The synchronization base could not be reconstructed.
    MissingSynchronizationBase,
}

/// Result of a merge followed by deterministic rerender preparation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownSynchronizationResult {
    /// Semantic merge result requiring application-layer handling.
    pub outcome: ThreeWayOutcome,
    /// Rerendered bytes when the merge is safe to write.
    pub rendered: Option<Vec<u8>>,
    /// Snapshot that becomes the synchronization base after a successful write.
    pub synchronization_base: Option<SnapshotId>,
    /// Whether preserving the external observation and retrying is safe.
    pub retryable: bool,
}

/// Merges an external Markdown candidate with current canonical state.
///
/// # Parameters
/// - `base`: Last synchronized semantic document, when retrievable.
/// - `current`: Current canonical semantic document.
/// - `external`: Parsed external semantic candidate.
///
/// # Returns
/// A formatting-only result, merged result, convergence, structured conflict,
/// or explicit missing-base outcome.
#[must_use]
pub fn three_way_merge(
    base: Option<&MarkdownDocument>,
    current: &MarkdownDocument,
    external: &MarkdownDocument,
) -> ThreeWayOutcome {
    let Some(base) = base else {
        return ThreeWayOutcome::MissingSynchronizationBase;
    };
    if external == base {
        return ThreeWayOutcome::FormattingOnly {
            document: current.clone(),
        };
    }
    if current == base {
        return ThreeWayOutcome::Merged {
            document: external.clone(),
        };
    }
    if current == external {
        return ThreeWayOutcome::Converged {
            document: current.clone(),
        };
    }
    let conflicts = detect_conflicts(base, current, external);
    if !conflicts.is_empty() {
        return ThreeWayOutcome::Conflict { conflicts };
    }
    let mut merged = current.blocks.clone();
    insert_external_only(&mut merged, base, current, external);
    ThreeWayOutcome::Merged {
        document: MarkdownDocument::new(merged),
    }
}

/// Merges and prepares a Markdown projection for atomic rerender/write.
///
/// # Parameters
/// - `base_snapshot`: Last synchronized snapshot, when retrievable.
/// - `base`: Last synchronized semantic document, when retrievable.
/// - `current_snapshot`: Current canonical snapshot.
/// - `current`: Current canonical semantic document.
/// - `external`: Parsed external candidate.
/// - `style`: Exact deterministic Markdown style.
/// - `result_snapshot`: Snapshot to record after a successful merge promotion.
///
/// # Returns
/// Merge outcome, optional rendered bytes, and a deferred synchronization base.
///
/// # Errors
/// This preparation function does not perform I/O and does not return errors.
#[must_use]
pub fn synchronize(
    base_snapshot: Option<SnapshotId>,
    base: Option<&MarkdownDocument>,
    current_snapshot: SnapshotId,
    current: &MarkdownDocument,
    external: &MarkdownDocument,
    style: &MarkdownStyle,
    result_snapshot: SnapshotId,
) -> MarkdownSynchronizationResult {
    let outcome = three_way_merge(base, current, external);
    let safe_document = match &outcome {
        ThreeWayOutcome::FormattingOnly { document }
        | ThreeWayOutcome::Merged { document }
        | ThreeWayOutcome::Converged { document } => Some(document),
        ThreeWayOutcome::Conflict { .. } | ThreeWayOutcome::MissingSynchronizationBase => None,
    };
    let rendered = safe_document.map(|document| render_document(document, style).into_bytes());
    let synchronization_base = if rendered.is_some() {
        Some(result_snapshot)
    } else {
        base_snapshot.or(Some(current_snapshot))
    };
    MarkdownSynchronizationResult {
        retryable: rendered.is_none(),
        outcome,
        rendered,
        synchronization_base,
    }
}

/// Detects incompatible changes to the same base block.
///
/// # Parameters
/// - `base`: Last synchronized document.
/// - `current`: Current canonical document.
/// - `external`: External candidate document.
///
/// # Returns
/// Structured conflicts for incompatible changes.
fn detect_conflicts(
    base: &MarkdownDocument,
    current: &MarkdownDocument,
    external: &MarkdownDocument,
) -> Vec<MarkdownConflict> {
    let mut conflicts = Vec::new();
    for (index, base_block) in base.blocks.iter().enumerate() {
        let current_block = current.blocks.get(index);
        let external_block = external.blocks.get(index);
        let current_changed = current_block != Some(base_block);
        let external_changed = external_block != Some(base_block);
        if current_changed && external_changed && current_block != external_block {
            let kind = if current_block.is_none() || external_block.is_none() {
                MarkdownConflictKind::DeleteModify
            } else {
                MarkdownConflictKind::SameBlockChanged
            };
            conflicts.push(MarkdownConflict {
                kind,
                path: vec![index],
                base: Some(base_block.clone()),
                current: current_block.cloned(),
                external: external_block.cloned(),
            });
        }
    }
    conflicts
}

/// Inserts external blocks that are not present in the current state.
///
/// # Parameters
/// - `merged`: Destination merged block sequence.
/// - `base`: Last synchronized document.
/// - `current`: Current canonical document.
/// - `external`: External candidate document.
fn insert_external_only(
    merged: &mut Vec<MarkdownBlock>,
    base: &MarkdownDocument,
    current: &MarkdownDocument,
    external: &MarkdownDocument,
) {
    for (index, block) in external.blocks.iter().enumerate() {
        if base.blocks.contains(block) || current.blocks.contains(block) {
            continue;
        }
        let position = index.min(merged.len());
        merged.insert(position, block.clone());
    }
}
