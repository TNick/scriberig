//! Performs deterministic, context-aware matching of Markdown nodes.

use scriberig_core::EntityId;

use crate::match_nodes_support::{
    candidate_context, flatten, nearby_anchor, parent_matches, same_context, same_kind,
    section_precedes_heading_anchor, text_similarity,
};
use crate::model::{MarkdownBlock, MarkdownDocument};

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
    /// Exact domain-significant section heading or title.
    DomainName,
    /// Same kind at an unchanged structural position.
    StructuralContext,
    /// Same sibling collection position with compatible surrounding context.
    SiblingAlignment,
    /// Same-kind node retained by an unchanged neighboring anchor.
    NearbyAnchor,
    /// Same-kind node selected by a unique conservative text score.
    TextSimilarity,
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
    let mut matches = Vec::new();
    for (path, block) in &candidates {
        matches.push(match_candidate(
            base,
            &mut used,
            &matches,
            &candidates,
            path,
            block,
        ));
    }
    MatchReport {
        matches,
        unmatched: base
            .iter()
            .enumerate()
            .filter(|(index, _)| !used[*index])
            .map(|(_, node)| node.id)
            .collect(),
    }
}

/// Matches one candidate block against available canonical context.
#[allow(clippy::too_many_lines)]
/// - `base`: Argument named `base`.
/// - `used`: Argument named `used`.
/// - `results`: Argument named `results`.
/// - `candidates`: Argument named `candidates`.
/// - `MarkdownBlock`: Argument named `MarkdownBlock`.
///
/// # Returns
/// The value produced by the declaration.
fn match_candidate(
    base: &[CanonicalNode],
    used: &mut [bool],
    results: &[NodeMatch],
    candidates: &[(Vec<usize>, MarkdownBlock)],
    path: &[usize],
    block: &MarkdownBlock,
) -> NodeMatch {
    let (mapped_parent, parent_is_new) = candidate_context(path, results);
    let root_context = base
        .iter()
        .any(|node| node.parent.is_none() && same_kind(&node.block, block));
    let eligible = |index: usize, node: &CanonicalNode| {
        !used[index]
            && !parent_is_new
            && parent_matches(node.parent, mapped_parent, path, root_context)
    };
    let exact_content: Vec<_> = base
        .iter()
        .enumerate()
        .filter(|(index, node)| eligible(*index, node) && node.block == *block)
        .map(|(index, _)| index)
        .collect();
    if exact_content.len() > 1 {
        return ambiguous(path, base, &exact_content);
    }
    let exact_path: Vec<_> = base
        .iter()
        .enumerate()
        .filter(|(index, node)| {
            eligible(*index, node)
                && node.position == path.last().copied().unwrap_or(0)
                && node.block == *block
        })
        .map(|(index, _)| index)
        .collect();
    if exact_path.len() == 1 {
        return matched(
            path,
            base,
            used,
            exact_path[0],
            MatchEvidence::ExactStructural,
        );
    }
    if exact_content.len() == 1 {
        let evidence = if matches!(block, MarkdownBlock::Section(_)) {
            MatchEvidence::DomainName
        } else {
            MatchEvidence::ExactContent
        };
        return matched(path, base, used, exact_content[0], evidence);
    }
    let contextual: Vec<_> = base
        .iter()
        .enumerate()
        .filter(|(index, node)| {
            eligible(*index, node)
                && node.position == path.last().copied().unwrap_or(0)
                && same_context(&node.block, block)
                && !section_precedes_heading_anchor(base, candidates, path, block)
        })
        .map(|(index, _)| index)
        .collect();
    if contextual.len() == 1 {
        return matched(
            path,
            base,
            used,
            contextual[0],
            MatchEvidence::SiblingAlignment,
        );
    }
    let nearby: Vec<_> = base
        .iter()
        .enumerate()
        .filter(|(index, node)| {
            eligible(*index, node)
                && same_context(&node.block, block)
                && nearby_anchor(base, results, candidates, path, node)
        })
        .map(|(index, _)| index)
        .collect();
    if nearby.len() == 1 {
        return matched(path, base, used, nearby[0], MatchEvidence::NearbyAnchor);
    }
    if nearby.len() > 1 {
        return ambiguous(path, base, &nearby);
    }
    let same_kind: Vec<_> = base
        .iter()
        .enumerate()
        .filter(|(index, node)| {
            eligible(*index, node)
                && same_context(&node.block, block)
                && !section_precedes_heading_anchor(base, candidates, path, block)
        })
        .map(|(index, _)| index)
        .collect();
    let scored: Vec<_> = same_kind
        .iter()
        .map(|index| (*index, text_similarity(&base[*index].block, block)))
        .filter(|(_, (shared, union))| shared.saturating_mul(2) >= *union)
        .collect();
    if let Some((index, (shared, union))) = scored.first().copied()
        && scored.get(1).is_none_or(|(_, (next_shared, next_union))| {
            shared.saturating_mul(*next_union) > next_shared.saturating_mul(union)
        })
    {
        return matched(path, base, used, index, MatchEvidence::TextSimilarity);
    }
    match same_kind.as_slice() {
        [index] => matched(path, base, used, *index, MatchEvidence::StructuralContext),
        [] => NodeMatch::New {
            candidate_path: path.to_owned(),
        },
        _ => ambiguous(path, base, &same_kind),
    }
}

/// Creates a matched result and marks its canonical identity as consumed.
/// - `path`: Argument named `path`.
/// - `base`: Argument named `base`.
/// - `used`: Argument named `used`.
/// - `index`: Argument named `index`.
/// - `evidence`: Argument named `evidence`.
///
/// # Returns
/// The value produced by the declaration.
fn matched(
    path: &[usize],
    base: &[CanonicalNode],
    used: &mut [bool],
    index: usize,
    evidence: MatchEvidence,
) -> NodeMatch {
    used[index] = true;
    NodeMatch::Matched {
        candidate_path: path.to_owned(),
        existing: base[index].id,
        evidence,
    }
}

/// Creates an ambiguity result for equally plausible canonical identities.
/// - `path`: Argument named `path`.
/// - `base`: Argument named `base`.
/// - `indexes`: Argument named `indexes`.
///
/// # Returns
/// The value produced by the declaration.
fn ambiguous(path: &[usize], base: &[CanonicalNode], indexes: &[usize]) -> NodeMatch {
    NodeMatch::Ambiguous {
        candidate_path: path.to_owned(),
        possible: indexes.iter().map(|index| base[*index].id).collect(),
    }
}
