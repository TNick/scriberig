//! Provides structural and text evidence helpers for Markdown node matching.

use scriberig_core::EntityId;

use crate::{CanonicalNode, MarkdownBlock, MarkdownDocument, NodeMatch};

/// Checks whether an unchanged neighboring candidate anchors a base node.
///
/// # Parameters
/// - `base`: Canonical nodes from the base snapshot.
/// - `results`: Matches established for preceding candidates.
/// - `candidates`: Candidate paths and blocks.
/// - `path`: Structural path being matched.
/// - `node`: Base node being considered.
///
/// # Returns
/// The value produced by the declaration.
pub(crate) fn nearby_anchor(
    base: &[CanonicalNode],
    results: &[NodeMatch],
    candidates: &[(Vec<usize>, MarkdownBlock)],
    path: &[usize],
    node: &CanonicalNode,
) -> bool {
    let Some(index) = path.last().copied() else {
        return false;
    };
    let parent_path = &path[..path.len().saturating_sub(1)];
    candidates.iter().any(|(candidate_path, candidate_block)| {
        let Some(anchor_index) = candidate_path.last().copied() else {
            return false;
        };
        if candidate_path == path
            || candidate_path[..candidate_path.len().saturating_sub(1)] != *parent_path
            || index.abs_diff(anchor_index) != 1
        {
            return false;
        }
        let Some(NodeMatch::Matched { existing, .. }) = results.iter().find(|item| {
            matches!(item, NodeMatch::Matched { candidate_path: path, .. } if path == candidate_path)
        }) else {
            return false;
        };
        let Some(anchor) = base.iter().find(|candidate| candidate.id == *existing) else {
            return false;
        };
        anchor.parent == node.parent
            && anchor.position.abs_diff(node.position) <= 1
            && anchor.block == *candidate_block
    })
}

/// Computes conservative word-overlap similarity for same-kind text blocks.
///
/// # Parameters
/// - `left`: First semantic block.
/// - `right`: Second semantic block.
///
/// # Returns
/// The value produced by the declaration.
pub(crate) fn text_similarity(left: &MarkdownBlock, right: &MarkdownBlock) -> (usize, usize) {
    let left = block_text(left);
    let right = block_text(right);
    if left.is_empty() || right.is_empty() {
        return (0, 1);
    }
    let left_words: std::collections::BTreeSet<_> = left.split_whitespace().collect();
    let right_words: std::collections::BTreeSet<_> = right.split_whitespace().collect();
    let shared = left_words.intersection(&right_words).count();
    let union = left_words.union(&right_words).count();
    (shared, union)
}

/// Extracts comparable author text from a Markdown block.
/// - `block`: Argument named `block`.
///
/// # Returns
/// The value produced by the declaration.
fn block_text(block: &MarkdownBlock) -> String {
    match block {
        MarkdownBlock::Heading { text, .. }
        | MarkdownBlock::Paragraph(text)
        | MarkdownBlock::CodeBlock { text, .. } => text.clone(),
        MarkdownBlock::Section(section) => section.heading.clone(),
        MarkdownBlock::Link { label, target } => format!("{label} {target}"),
        MarkdownBlock::Image { alt, target } => format!("{alt} {target}"),
        MarkdownBlock::List { items, .. } => items
            .iter()
            .flat_map(|item| item.blocks.iter())
            .map(block_text)
            .collect::<Vec<_>>()
            .join(" "),
        MarkdownBlock::QuoteBlock(blocks) => {
            blocks.iter().map(block_text).collect::<Vec<_>>().join(" ")
        }
        MarkdownBlock::Table { headers, rows } => headers
            .iter()
            .chain(rows.iter().flat_map(|row| row.iter()))
            .cloned()
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// Detects a likely insertion before a later unchanged section heading.
///
/// # Parameters
/// - `base`: Canonical nodes from the base snapshot.
/// - `candidates`: Candidate paths and blocks.
/// - `path`: Candidate structural path.
/// - `block`: Candidate semantic block.
///
/// # Returns
/// The value produced by the declaration.
pub(crate) fn section_precedes_heading_anchor(
    base: &[CanonicalNode],
    candidates: &[(Vec<usize>, MarkdownBlock)],
    path: &[usize],
    block: &MarkdownBlock,
) -> bool {
    let MarkdownBlock::Section(candidate_section) = block else {
        return false;
    };
    let has_base_heading = base.iter().any(|node| {
        matches!(
            &node.block,
            MarkdownBlock::Section(section)
                if section.level == candidate_section.level
                    && section.heading == candidate_section.heading
        )
    });
    if has_base_heading {
        return false;
    }
    base.iter().any(|node| {
        let MarkdownBlock::Section(base_section) = &node.block else {
            return false;
        };
        base_section.level == candidate_section.level
            && node.position >= path.last().copied().unwrap_or(0)
            && candidates.iter().any(|(candidate_path, candidate)| {
                candidate_path != path
                    && matches!(
                        candidate,
                        MarkdownBlock::Section(section)
                            if section.level == base_section.level
                                && section.heading == base_section.heading
                    )
            })
    })
}

/// Checks whether a canonical node belongs to the candidate's known parent.
///
/// # Parameters
/// - `node_parent`: Base node's parent identity.
/// - `mapped_parent`: Matched candidate parent identity.
/// - `path`: Candidate structural path.
/// - `root_context`: Whether matching requires a canonical root.
///
/// # Returns
/// The value produced by the declaration.
pub(crate) fn parent_matches(
    node_parent: Option<EntityId>,
    mapped_parent: Option<EntityId>,
    path: &[usize],
    root_context: bool,
) -> bool {
    if path.len() == 1 {
        !root_context || node_parent.is_none()
    } else {
        mapped_parent.is_some_and(|parent| node_parent == Some(parent))
    }
}

/// Resolves the known parent context for one candidate path.
///
/// # Parameters
/// - `path`: Candidate structural path.
/// - `results`: Matches established for candidate ancestors.
///
/// # Returns
/// The value produced by the declaration.
pub(crate) fn candidate_context(path: &[usize], results: &[NodeMatch]) -> (Option<EntityId>, bool) {
    if path.len() <= 1 {
        return (None, false);
    }
    let parent_path = &path[..path.len() - 1];
    let mapped_parent = results.iter().find_map(|item| match item {
        NodeMatch::Matched {
            candidate_path,
            existing,
            ..
        } if candidate_path == parent_path => Some(*existing),
        _ => None,
    });
    let parent_is_new = results.iter().any(|item| match item {
        NodeMatch::New { candidate_path } | NodeMatch::Ambiguous { candidate_path, .. } => {
            parent_path.starts_with(candidate_path)
        }
        NodeMatch::Matched { .. } => false,
    });
    (mapped_parent, parent_is_new)
}

/// Flattens nested semantic blocks into stable structural paths.
///
/// # Parameters
/// - `document`: Semantic Markdown document to flatten.
///
/// # Returns
/// The value produced by the declaration.
pub(crate) fn flatten(document: &MarkdownDocument) -> Vec<(Vec<usize>, MarkdownBlock)> {
    let mut output = Vec::new();
    for (index, block) in document.blocks.iter().enumerate() {
        flatten_block(block, &[index], &mut output);
    }
    output
}

/// Adds a block and its nested sections to the flattened representation.
/// - `block`: Argument named `block`.
/// - `path`: Argument named `path`.
/// - `output`: Argument named `output`.
/// - `MarkdownBlock`: Argument named `MarkdownBlock`.
fn flatten_block(
    block: &MarkdownBlock,
    path: &[usize],
    output: &mut Vec<(Vec<usize>, MarkdownBlock)>,
) {
    output.push((path.to_owned(), block.clone()));
    if let MarkdownBlock::Section(section) = block {
        for (index, child) in section.blocks.iter().enumerate() {
            let mut child_path = path.to_owned();
            child_path.push(index);
            flatten_block(child, &child_path, output);
        }
    }
}

/// Reports whether two blocks have the same semantic kind.
///
/// # Parameters
/// - `left`: First semantic block.
/// - `right`: Second semantic block.
///
/// # Returns
/// The value produced by the declaration.
pub(crate) fn same_kind(left: &MarkdownBlock, right: &MarkdownBlock) -> bool {
    std::mem::discriminant(left) == std::mem::discriminant(right)
}

/// Reports whether two blocks share enough stable context for a changed edit.
///
/// # Parameters
/// - `left`: First semantic block.
/// - `right`: Second semantic block.
///
/// # Returns
/// The value produced by the declaration.
pub(crate) fn same_context(left: &MarkdownBlock, right: &MarkdownBlock) -> bool {
    match (left, right) {
        (MarkdownBlock::Section(left), MarkdownBlock::Section(right)) => left.level == right.level,
        _ => same_kind(left, right),
    }
}
