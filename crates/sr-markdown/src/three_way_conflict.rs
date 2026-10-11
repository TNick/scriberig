//! Builds structured Markdown merge conflicts.

use super::{Hunk, MarkdownBlock, MarkdownConflict, MarkdownConflictKind};

/// Builds a structured conflict for overlapping base edits.
/// - `super`: Parent-module imports used by this helper.
/// - `path`: Argument named `path`.
/// - `base`: Argument named `base`.
/// - `current`: Argument named `current`.
/// - `external`: Argument named `external`.
///
/// # Returns
/// The value produced by the declaration.
pub(super) fn make_conflict(
    path: &[usize],
    base: &[MarkdownBlock],
    current: &Hunk,
    external: &Hunk,
) -> MarkdownConflict {
    let mut conflict_path = path.to_vec();
    conflict_path.push(current.start);
    MarkdownConflict {
        kind: if current.replacement.is_empty() || external.replacement.is_empty() {
            MarkdownConflictKind::DeleteModify
        } else if current.start == current.end {
            MarkdownConflictKind::Structural
        } else {
            MarkdownConflictKind::SameBlockChanged
        },
        path: conflict_path,
        base: base.get(current.start).cloned(),
        current: current.replacement.first().cloned(),
        external: external.replacement.first().cloned(),
    }
}
