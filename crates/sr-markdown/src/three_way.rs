//! Merges Markdown semantic states using an explicit base/current/external triplet.

use crate::{MarkdownBlock, MarkdownDocument, MarkdownStyle, equivalent, parse, render_document};
use scriberig_core::SnapshotId;

#[path = "three_way_conflict.rs"]
mod three_way_conflict;
use three_way_conflict::make_conflict;

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
/// - `base`: Last synchronized semantic document, when retrievable.
/// - `current`: Current canonical semantic document.
/// - `external`: Parsed external semantic candidate.
///
/// A formatting-only result, merged result, convergence, structured conflict,
/// or explicit missing-base outcome.
#[must_use]
/// # Returns
/// The operation result.
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
    match merge_sequence(&base.blocks, &current.blocks, &external.blocks, &[]) {
        Ok(blocks) => ThreeWayOutcome::Merged {
            document: MarkdownDocument::new(blocks),
        },
        Err(conflicts) => ThreeWayOutcome::Conflict { conflicts },
    }
}

/// Merges and prepares a Markdown projection for atomic rerender/write.
///
/// - `base_snapshot`: Last synchronized snapshot, when retrievable.
/// - `base`: Last synchronized semantic document, when retrievable.
/// - `current_snapshot`: Current canonical snapshot.
/// - `current`: Current canonical semantic document.
/// - `external`: Parsed external candidate.
/// - `style`: Exact deterministic Markdown style.
/// - `result_snapshot`: Snapshot to record after a successful merge promotion.
///
/// Merge outcome, optional rendered bytes, and a deferred synchronization base.
///
/// This preparation function does not perform I/O and does not return errors.
#[must_use]
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
    let rendered = safe_document.and_then(|document| {
        let bytes = render_document(document, style).into_bytes();
        let source = std::str::from_utf8(&bytes).ok()?;
        let candidate = parse(source).ok()?;
        equivalent(document, &candidate.document).then_some(bytes)
    });
    let outcome = if rendered.is_none()
        && matches!(
            &outcome,
            ThreeWayOutcome::FormattingOnly { .. }
                | ThreeWayOutcome::Merged { .. }
                | ThreeWayOutcome::Converged { .. }
        ) {
        ThreeWayOutcome::Conflict {
            conflicts: vec![MarkdownConflict {
                kind: MarkdownConflictKind::Structural,
                path: Vec::new(),
                base: None,
                current: None,
                external: None,
            }],
        }
    } else {
        outcome
    };
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

/// Describes one side's replacement relative to a synchronization base.
#[derive(Clone, Debug)]
struct Hunk {
    /// Inclusive base start.
    start: usize,
    /// Exclusive base end.
    end: usize,
    /// Replacement blocks.
    replacement: Vec<MarkdownBlock>,
}

/// Merges a block sequence using exact unchanged base anchors.
/// - `base`: Argument named `base`.
/// - `current`: Argument named `current`.
/// - `external`: Argument named `external`.
/// - `path`: Argument named `path`.
///
///
/// Returns an error when the operation cannot complete.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn merge_sequence(
    base: &[MarkdownBlock],
    current: &[MarkdownBlock],
    external: &[MarkdownBlock],
    path: &[usize],
) -> Result<Vec<MarkdownBlock>, Vec<MarkdownConflict>> {
    let current_hunks = diff_hunks(base, current);
    let external_hunks = diff_hunks(base, external);
    let mut merged = Vec::new();
    let mut conflicts = Vec::new();
    let mut cursor = 0;
    let mut current_index = 0;
    let mut external_index = 0;
    while cursor <= base.len() {
        let next_start = [
            current_hunks
                .get(current_index)
                .map_or(usize::MAX, |hunk| hunk.start),
            external_hunks
                .get(external_index)
                .map_or(usize::MAX, |hunk| hunk.start),
        ]
        .into_iter()
        .min()
        .unwrap_or(usize::MAX);
        if next_start == usize::MAX {
            merged.extend_from_slice(&base[cursor..]);
            break;
        }
        merged.extend_from_slice(&base[cursor..next_start]);
        let left = current_hunks
            .get(current_index)
            .filter(|hunk| hunk.start == next_start);
        let right = external_hunks
            .get(external_index)
            .filter(|hunk| hunk.start == next_start);
        match (left, right) {
            (Some(left), Some(right)) if left.end == right.end => {
                if left.replacement == right.replacement {
                    merged.extend(left.replacement.clone());
                } else if let Some(section) = merge_section_replacement(base, left, right, path)? {
                    merged.push(section);
                } else {
                    conflicts.push(make_conflict(path, base, left, right));
                }
                cursor = left.end;
                current_index += 1;
                external_index += 1;
            }
            (Some(left), Some(right)) => {
                conflicts.push(make_conflict(path, base, left, right));
                cursor = left.end.max(right.end);
                current_index += 1;
                external_index += 1;
            }
            (Some(hunk), None) => {
                merged.extend(hunk.replacement.clone());
                cursor = hunk.end;
                current_index += 1;
            }
            (None, Some(hunk)) => {
                merged.extend(hunk.replacement.clone());
                cursor = hunk.end;
                external_index += 1;
            }
            (None, None) => unreachable!("a next hunk must exist"),
        }
    }
    if !conflicts.is_empty() {
        return Err(conflicts);
    }
    Ok(merged)
}

/// Merges two edits that both changed the same stable section container.
/// - `base`: Argument named `base`.
/// - `current`: Argument named `current`.
/// - `external`: Argument named `external`.
/// - `path`: Argument named `path`.
///
///
/// Returns an error when the operation cannot complete.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn merge_section_replacement(
    base: &[MarkdownBlock],
    current: &Hunk,
    external: &Hunk,
    path: &[usize],
) -> Result<Option<MarkdownBlock>, Vec<MarkdownConflict>> {
    let Some(MarkdownBlock::Section(base_section)) = base.get(current.start) else {
        return Ok(None);
    };
    let [MarkdownBlock::Section(current_section)] = current.replacement.as_slice() else {
        return Ok(None);
    };
    let [MarkdownBlock::Section(external_section)] = external.replacement.as_slice() else {
        return Ok(None);
    };
    if current_section.level != base_section.level
        || external_section.level != base_section.level
        || current_section.heading != base_section.heading
        || external_section.heading != base_section.heading
    {
        return Ok(None);
    }
    let mut child_path = path.to_vec();
    child_path.push(current.start);
    let blocks = merge_sequence(
        &base_section.blocks,
        &current_section.blocks,
        &external_section.blocks,
        &child_path,
    )?;
    Ok(Some(MarkdownBlock::Section(crate::Section {
        level: base_section.level,
        heading: base_section.heading.clone(),
        blocks,
    })))
}

/// Computes replacement hunks by retaining exact unchanged anchors.
/// - `base`: Argument named `base`.
/// - `side`: Argument named `side`.
///
/// # Returns
/// The operation result.
fn diff_hunks(base: &[MarkdownBlock], side: &[MarkdownBlock]) -> Vec<Hunk> {
    let mut anchors = Vec::new();
    let mut base_index = 0;
    let mut side_index = 0;
    while base_index < base.len() && side_index < side.len() {
        if base[base_index] == side[side_index] {
            anchors.push((base_index, side_index));
            base_index += 1;
            side_index += 1;
        } else if let Some(next_side) = side[side_index..]
            .iter()
            .position(|block| block == &base[base_index])
        {
            side_index += next_side;
        } else {
            base_index += 1;
        }
    }
    let mut hunks = Vec::new();
    let mut previous_base = 0;
    let mut previous_side = 0;
    for (anchor_base, anchor_side) in anchors {
        if previous_base != anchor_base || previous_side != anchor_side {
            hunks.push(Hunk {
                start: previous_base,
                end: anchor_base,
                replacement: side[previous_side..anchor_side].to_vec(),
            });
        }
        previous_base = anchor_base + 1;
        previous_side = anchor_side + 1;
    }
    if previous_base != base.len() || previous_side != side.len() {
        hunks.push(Hunk {
            start: previous_base,
            end: base.len(),
            replacement: side[previous_side..].to_vec(),
        });
    }
    hunks
}
