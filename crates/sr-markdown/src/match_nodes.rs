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
        let exact_content: Vec<_> = base
            .iter()
            .enumerate()
            .filter(|(index, node)| !used[*index] && node.block == *block)
            .map(|(index, _)| index)
            .collect();
        if exact_content.len() > 1 {
            results.push(NodeMatch::Ambiguous {
                candidate_path: path.clone(),
                possible: exact_content.iter().map(|index| base[*index].id).collect(),
            });
            continue;
        }
        let exact_path: Vec<_> = base
            .iter()
            .enumerate()
            .filter(|(index, node)| {
                !used[*index] && node.position == path.last().copied().unwrap_or(0)
            })
            .filter(|(_, node)| node.block == *block)
            .map(|(index, _)| index)
            .collect();
        if exact_path.len() == 1 {
            let index = exact_path[0];
            used[index] = true;
            results.push(NodeMatch::Matched {
                candidate_path: path.clone(),
                existing: base[index].id,
                evidence: MatchEvidence::ExactStructural,
            });
            continue;
        }
        if exact_content.len() == 1 {
            let index = exact_content[0];
            used[index] = true;
            results.push(NodeMatch::Matched {
                candidate_path: path.clone(),
                existing: base[index].id,
                evidence: MatchEvidence::ExactContent,
            });
            continue;
        }
        if exact_content.len() > 1 {
            results.push(NodeMatch::Ambiguous {
                candidate_path: path.clone(),
                possible: exact_content.iter().map(|index| base[*index].id).collect(),
            });
            continue;
        }
        let same_kind: Vec<_> = base
            .iter()
            .enumerate()
            .filter(|(index, node)| !used[*index] && same_kind(&node.block, block))
            .map(|(index, _)| index)
            .collect();
        match same_kind.len() {
            1 => {
                let index = same_kind[0];
                used[index] = true;
                results.push(NodeMatch::Matched {
                    candidate_path: path.clone(),
                    existing: base[index].id,
                    evidence: MatchEvidence::StructuralContext,
                });
            }
            count if count > 1 => results.push(NodeMatch::Ambiguous {
                candidate_path: path.clone(),
                possible: same_kind.iter().map(|index| base[*index].id).collect(),
            }),
            _ => results.push(NodeMatch::New {
                candidate_path: path.clone(),
            }),
        }
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
