//! Places plan structure and continuation branches in the conversation tree.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use super::{ConversationRow, ConversationRowKind, ConversationSource};
use scriberig_core::EntityId;
use scriberig_session::{Plan, PlanNodeParent, Session};

/// Places each assigned worker branch beneath its durable plan node.
///
/// Existing session rows retain canonical branch and activity identities;
/// this presentation pass moves those rows under the plan node that owns them.
///
/// - `session`: Canonical session containing plan node branch assignments.
/// - `rows`: Structurally projected rows to update in place.
pub(super) fn relocate_plan_worker_branches(session: &Session, rows: &mut Vec<ConversationRow>) {
    for node in session.plans.iter().flat_map(|plan| &plan.nodes) {
        let Some(branch) = node.branch else {
            continue;
        };
        let Some(branch_index) = rows
            .iter()
            .position(|row| row.source == ConversationSource::Branch(branch))
        else {
            continue;
        };
        let Some(node_index) = rows
            .iter()
            .position(|row| row.source == ConversationSource::PlanNode(node.id))
        else {
            continue;
        };
        let branch_depth = rows[branch_index].depth;
        let end = rows[branch_index + 1..]
            .iter()
            .position(|row| row.depth <= branch_depth)
            .map_or(rows.len(), |offset| branch_index + 1 + offset);
        if (branch_index..end).contains(&node_index) {
            continue;
        }
        let mut subtree = rows.drain(branch_index..end).collect::<Vec<_>>();
        let target_depth = rows
            .iter()
            .find(|row| row.source == ConversationSource::PlanNode(node.id))
            .map_or(0, |row| row.depth.saturating_add(1));
        for row in &mut subtree {
            row.depth = if row.depth >= branch_depth {
                target_depth.saturating_add(row.depth - branch_depth)
            } else {
                target_depth.saturating_sub(branch_depth - row.depth)
            };
        }
        let insertion = rows
            .iter()
            .position(|row| row.source == ConversationSource::PlanNode(node.id))
            .map_or(rows.len(), |index| index + 1);
        rows.splice(insertion..insertion, subtree);
    }
}

/// Moves an exact-anchor continuation subtree below the activity it continues.
///
/// - `anchor`: Stable historical continuation location.
/// - `branch`: Continuation branch to relocate.
/// - `rows`: Structural projection rows.
pub(super) fn relocate_continuation(
    anchor: scriberig_session::ContinuationAnchor,
    branch: EntityId,
    rows: &mut Vec<ConversationRow>,
) {
    let Some(start) = rows
        .iter()
        .position(|row| row.source == ConversationSource::Branch(branch))
    else {
        return;
    };
    let depth = rows[start].depth;
    let end = rows[start + 1..]
        .iter()
        .position(|row| row.depth <= depth)
        .map_or(rows.len(), |offset| start + 1 + offset);
    let subtree = rows.drain(start..end).collect::<Vec<_>>();
    let (anchor_branch, activity) = match anchor {
        scriberig_session::ContinuationAnchor::BranchHead { branch } => (branch, None),
        scriberig_session::ContinuationAnchor::AssistantOutput { branch, turn }
        | scriberig_session::ContinuationAnchor::HumanInput { branch, turn } => {
            (branch, Some(ConversationSource::Activity(turn)))
        }
        scriberig_session::ContinuationAnchor::BranchResult { branch, result } => {
            (branch, Some(ConversationSource::BranchResult(result)))
        }
    };
    let Some(parent) = rows
        .iter()
        .position(|row| row.source == ConversationSource::Branch(anchor_branch))
    else {
        rows.splice(start.min(rows.len())..start.min(rows.len()), subtree);
        return;
    };
    let insertion = activity
        .and_then(|source| rows.iter().position(|row| row.source == source))
        .map_or_else(
            || {
                let parent_depth = rows[parent].depth;
                rows[parent + 1..]
                    .iter()
                    .position(|row| {
                        row.kind == ConversationRowKind::Branch && row.depth <= parent_depth + 1
                    })
                    .map_or(rows.len(), |offset| parent + 1 + offset)
            },
            |index| index.saturating_add(1),
        );
    let insertion = insertion.min(rows.len());
    rows.splice(insertion..insertion, subtree);
}

/// Adds one plan and its explicit structural node tree to the projection.
///
/// - `plan`: Durable plan to project.
/// - `rows`: Conversation rows receiving the plan structure.
/// # Parameters
/// - `session`: Input for this operation.
pub(super) fn append_plan(session: &Session, plan: &Plan, rows: &mut Vec<ConversationRow>) {
    let anchor = plan.anchor;
    let anchor_branch = anchor.as_ref().map(|anchor| match anchor {
        scriberig_session::ContinuationAnchor::BranchHead { branch }
        | scriberig_session::ContinuationAnchor::AssistantOutput { branch, .. }
        | scriberig_session::ContinuationAnchor::HumanInput { branch, .. }
        | scriberig_session::ContinuationAnchor::BranchResult { branch, .. } => *branch,
    });
    let parent_index = anchor_branch
        .and_then(|branch| {
            rows.iter()
                .position(|row| row.source == ConversationSource::Branch(branch))
        })
        .unwrap_or(0);
    let depth = rows
        .get(parent_index)
        .map_or(1, |row| row.depth.saturating_add(1));
    let anchor_index = anchor
        .and_then(|anchor| rows.iter().position(|row| row.reply_anchor == Some(anchor)))
        .unwrap_or(parent_index);
    let insertion = anchor_index.saturating_add(1);
    rows.insert(
        insertion,
        ConversationRow {
            source: ConversationSource::Plan(plan.id),
            reply_anchor: None,
            depth,
            label: format!("Plan: {} · {:?}", plan.goal, plan.status),
            kind: ConversationRowKind::Plan,
        },
    );
    let mut children = BTreeMap::<Option<EntityId>, Vec<&scriberig_session::PlanNode>>::new();
    for node in &plan.nodes {
        let parent = match node.parent {
            Some(PlanNodeParent::Branch(branch)) => Some(branch),
            Some(PlanNodeParent::Node(parent)) => Some(parent),
            None => None,
        };
        children.entry(parent).or_default().push(node);
    }
    let mut emitted = BTreeSet::new();
    let mut nodes = Vec::new();
    append_plan_children(
        session,
        plan,
        anchor_branch,
        depth.saturating_add(1),
        &children,
        &mut emitted,
        &mut nodes,
    );
    rows.splice(
        insertion.saturating_add(1)..insertion.saturating_add(1),
        nodes,
    );
}

/// Recursively appends plan nodes in their durable vector order.
///
/// - `plan`: Owning durable plan.
/// - `parent`: Branch or plan-node identity being expanded.
/// - `depth`: Derived plan-tree depth.
/// - `children`: Node index grouped by structural parent.
/// - `emitted`: Node identities already projected.
/// - `rows`: Output rows being built.
/// # Parameters
/// - `session`: Input for this operation.
pub(super) fn append_plan_children(
    session: &Session,
    plan: &Plan,
    parent: Option<EntityId>,
    depth: usize,
    children: &BTreeMap<Option<EntityId>, Vec<&scriberig_session::PlanNode>>,
    emitted: &mut BTreeSet<EntityId>,
    rows: &mut Vec<ConversationRow>,
) {
    let Some(nodes) = children.get(&parent) else {
        return;
    };
    for node in nodes {
        if !emitted.insert(node.id) {
            continue;
        }
        let waits = plan
            .dependencies
            .iter()
            .filter(|dependency| {
                dependency.dependent == node.id && dependency.fulfilled_by.is_none()
            })
            .map(|dependency| dependency.prerequisite.to_string())
            .collect::<Vec<_>>();
        let mut label = format!("{:?} · {}", node.status, node.title);
        if let Some(assignment) = &node.assignment {
            let _ = write!(label, " · {}", assignment.profile);
        }
        if let Some(result_id) = node.result {
            if let Some(summary) = session
                .branches
                .iter()
                .filter_map(|branch| branch.result.as_ref())
                .find(|result| result.id == result_id)
                .and_then(|result| result.summary.as_deref())
            {
                let _ = write!(label, " · {summary}");
            }
        }
        if !waits.is_empty() {
            let _ = write!(label, " · waits: {}", waits.join(", "));
        }
        rows.push(ConversationRow {
            source: ConversationSource::PlanNode(node.id),
            reply_anchor: None,
            depth,
            label,
            kind: ConversationRowKind::PlanNode,
        });
        append_plan_children(
            session,
            plan,
            Some(node.id),
            depth.saturating_add(1),
            children,
            emitted,
            rows,
        );
    }
}
