//! Performs deterministic, context-aware matching of Markdown nodes.

use scriberig_core::EntityId;

use crate::model::{MarkdownBlock, MarkdownDocument};

/// Flattened semantic blocks paired with structural paths.
type FlattenedBlocks = Vec<(Vec<usize>, MarkdownBlock)>;

/// A canonical node participating in one projected semantic surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalNode {
    /// Stable canonical identity.
    pub id: EntityId,
    /// Parent collection identity, when the node is nested.
    pub parent: Option<EntityId>,
    /// Semantic block represented by the node.
    pub block: MarkdownBlock,
    /// Semantic order within its parent collection.
    pub position: usize,
}

/// Identifies why a candidate was matched to a canonical node.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatchEvidence {
    /// Same structural path and exact semantic content.
    ExactStructural,
    /// Exact content with a unique local candidate.
    ExactContent,
    /// Same kind at an unchanged structural position.
    StructuralContext,
}

/// One candidate matching result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeMatch {
    /// Candidate maps to an existing canonical identity.
    Matched {
        /// Candidate path in the projected tree.
        candidate_path: Vec<usize>,
        /// Existing canonical node.
        existing: EntityId,
        /// Matching evidence.
        evidence: MatchEvidence,
    },
    /// Candidate has no safe predecessor.
    New {
        /// Candidate path in the projected tree.
        candidate_path: Vec<usize>,
    },
    /// More than one predecessor remains equally plausible.
    Ambiguous {
        /// Candidate path in the projected tree.
        candidate_path: Vec<usize>,
        /// Possible canonical identities.
        possible: Vec<EntityId>,
    },
}

/// Complete matching evidence, including canonical nodes absent from a candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchReport {
    /// Result for every candidate node.
    pub matches: Vec<NodeMatch>,
    /// Canonical nodes not represented by a candidate.
    pub unmatched: Vec<EntityId>,
}

/// Matches a candidate document against canonical nodes in deterministic phases.
///
/// # Parameters
/// - `base`: Canonical nodes in projected structural order.
/// - `candidate`: Parsed candidate document.
///
/// # Returns
/// Match evidence, explicit ambiguity, and unmatched canonical identities.
#[must_use]
pub fn match_nodes(base: &[CanonicalNode], candidate: &MarkdownDocument) -> MatchReport {
    let candidates = flatten(candidate);
    let mut used = vec![false; base.len()];
    let mut results = Vec::new();
    for (path, block) in &candidates {
        results.push(match_candidate(base, &mut used, &results, path, block));
    }
    MatchReport {
        matches: results,
        unmatched: base
            .iter()
            .enumerate()
            .filter(|(index, _)| !used[*index])
            .map(|(_, node)| node.id)
            .collect(),
    }
}

/// Matches one candidate block against the available canonical context.
///
/// # Parameters
/// - `base`: Canonical nodes available for matching.
/// - `used`: Mutable match markers for canonical nodes.
/// - `results`: Matches already established in document order.
/// - `path`: Candidate structural path.
/// - `block`: Candidate semantic block.
///
/// # Returns
/// The explicit match outcome for this candidate.
#[allow(clippy::too_many_lines)]
fn match_candidate(
    base: &[CanonicalNode],
    used: &mut [bool],
    results: &[NodeMatch],
    path: &[usize],
    block: &MarkdownBlock,
) -> NodeMatch {
    let (mapped_parent, parent_is_new) = candidate_context(path, results);
    let root_context = base
        .iter()
        .any(|node| node.parent.is_none() && same_kind(&node.block, block));
    let exact_content: Vec<_> = base
        .iter()
        .enumerate()
        .filter(|(index, node)| {
            !used[*index]
                && !parent_is_new
                && node.block == *block
                && parent_matches(node.parent, mapped_parent, path, root_context)
        })
        .map(|(index, _)| index)
        .collect();
    if exact_content.len() > 1 {
        return NodeMatch::Ambiguous {
            candidate_path: path.to_owned(),
            possible: exact_content.iter().map(|index| base[*index].id).collect(),
        };
    }
    let exact_path: Vec<_> = base
        .iter()
        .enumerate()
        .filter(|(index, node)| {
            !used[*index]
                && !parent_is_new
                && node.position == path.last().copied().unwrap_or(0)
                && parent_matches(node.parent, mapped_parent, path, root_context)
                && node.block == *block
        })
        .map(|(index, _)| index)
        .collect();
    if let Some(&index) = exact_path.first().filter(|_| exact_path.len() == 1) {
        used[index] = true;
        return NodeMatch::Matched {
            candidate_path: path.to_owned(),
            existing: base[index].id,
            evidence: MatchEvidence::ExactStructural,
        };
    }
    if let Some(&index) = exact_content.first().filter(|_| exact_content.len() == 1) {
        used[index] = true;
        return NodeMatch::Matched {
            candidate_path: path.to_owned(),
            existing: base[index].id,
            evidence: MatchEvidence::ExactContent,
        };
    }
    if exact_content.len() > 1 {
        return NodeMatch::Ambiguous {
            candidate_path: path.to_owned(),
            possible: exact_content.iter().map(|index| base[*index].id).collect(),
        };
    }
    let contextual: Vec<_> = base
        .iter()
        .enumerate()
        .filter(|(index, node)| {
            !used[*index]
                && !parent_is_new
                && node.position == path.last().copied().unwrap_or(0)
                && same_context(&node.block, block)
                && parent_matches(node.parent, mapped_parent, path, root_context)
        })
        .map(|(index, _)| index)
        .collect();
    if let Some(&index) = contextual.first().filter(|_| contextual.len() == 1) {
        used[index] = true;
        return NodeMatch::Matched {
            candidate_path: path.to_owned(),
            existing: base[index].id,
            evidence: MatchEvidence::StructuralContext,
        };
    }
    let same_kind: Vec<_> = base
        .iter()
        .enumerate()
        .filter(|(index, node)| !used[*index] && same_context(&node.block, block))
        .filter(|(_, node)| {
            !parent_is_new && parent_matches(node.parent, mapped_parent, path, root_context)
        })
        .map(|(index, _)| index)
        .collect();
    match same_kind.len() {
        1 => {
            let index = same_kind[0];
            used[index] = true;
            NodeMatch::Matched {
                candidate_path: path.to_owned(),
                existing: base[index].id,
                evidence: MatchEvidence::StructuralContext,
            }
        }
        count if count > 1 => NodeMatch::Ambiguous {
            candidate_path: path.to_owned(),
            possible: same_kind.iter().map(|index| base[*index].id).collect(),
        },
        _ => NodeMatch::New {
            candidate_path: path.to_owned(),
        },
    }
}

/// Checks whether a canonical node belongs to the candidate's known parent.
///
/// # Parameters
/// - `node_parent`: Canonical parent identity.
/// - `mapped_parent`: Parent identity established by an earlier match.
/// - `path`: Candidate structural path.
/// - `root_context`: Whether a top-level canonical node of this kind exists.
///
/// # Returns
/// `true` when the node occupies the candidate's known collection.
fn parent_matches(
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
/// - `results`: Matches already established in document order.
///
/// # Returns
/// The mapped parent identity and whether an ancestor is newly created.
fn candidate_context(path: &[usize], results: &[NodeMatch]) -> (Option<EntityId>, bool) {
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
/// - `document`: Document to flatten.
///
/// # Returns
/// Blocks paired with their structural paths.
fn flatten(document: &MarkdownDocument) -> FlattenedBlocks {
    let mut output = Vec::new();
    for (index, block) in document.blocks.iter().enumerate() {
        flatten_block(block, &[index], &mut output);
    }
    output
}

/// Adds a block and its nested children to the flattened representation.
///
/// # Parameters
/// - `block`: Block to add.
/// - `path`: Current structural path.
/// - `output`: Destination flattened list.
fn flatten_block(block: &MarkdownBlock, path: &[usize], output: &mut FlattenedBlocks) {
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
/// - `left`: First block.
/// - `right`: Second block.
///
/// # Returns
/// `true` when both blocks have the same variant.
fn same_kind(left: &MarkdownBlock, right: &MarkdownBlock) -> bool {
    std::mem::discriminant(left) == std::mem::discriminant(right)
}

/// Reports whether two blocks share enough stable context for a changed edit.
///
/// # Parameters
/// - `left`: Existing canonical block.
/// - `right`: Candidate block.
///
/// # Returns
/// `true` when structural matching may safely retain identity.
fn same_context(left: &MarkdownBlock, right: &MarkdownBlock) -> bool {
    match (left, right) {
        (MarkdownBlock::Section(left), MarkdownBlock::Section(right)) => {
            left.level == right.level && left.heading == right.heading
        }
        _ => same_kind(left, right),
    }
}
