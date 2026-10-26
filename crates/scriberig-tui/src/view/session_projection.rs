//! Projects nested branch graphs into session rows without recursive traversal.

use std::collections::{BTreeSet, HashMap};

use super::{SessionRow, SessionRowKind};
use scriberig_core::EntityId;
use scriberig_session::{Branch, BranchStatus, Session, SessionViews};

/// Appends the session's branches in stable preorder and returns safe tool details.
///
/// Child indexes are built once, so deep or wide branch graphs do not trigger a
/// recursive walk or a full-session scan for every branch.
///
/// - `session`: Canonical session graph to project.
/// - `expanded`: Completed child branches whose activity is visible.
/// - `rows`: Output rows receiving branch, activity, and result records.
///
/// # Returns
/// Safe tool invocation details keyed by their stable semantic identity.
pub(super) fn append_branches(
    session: &Session,
    expanded: &BTreeSet<EntityId>,
    rows: &mut Vec<SessionRow>,
) -> std::collections::BTreeMap<EntityId, String> {
    let mut by_id = HashMap::with_capacity(session.branches.len());
    let mut inferred_children = HashMap::<EntityId, Vec<EntityId>>::new();
    for branch in &session.branches {
        by_id.entry(branch.id).or_insert(branch);
        if let Some(parent) = branch.parent {
            inferred_children.entry(parent).or_default().push(branch.id);
        }
    }

    let mut visited = BTreeSet::new();
    let mut tool_invocation_details = std::collections::BTreeMap::new();
    let mut tree = BranchTreeState {
        by_id: &by_id,
        inferred_children: &inferred_children,
        visited: &mut visited,
        rows,
        tool_invocation_details: &mut tool_invocation_details,
        expanded,
    };
    for branch in session
        .branches
        .iter()
        .filter(|branch| branch.parent.is_none())
    {
        tree.append_branch_tree(branch.id, 1);
    }
    for branch in SessionViews::branches(session) {
        if !tree.visited.contains(&branch.id) {
            tree.append_branch_tree(branch.id, 1);
        }
    }
    tree.tool_invocation_details.clone()
}

/// Holds indexed branch data and output sinks for one stable tree projection.
///
/// # Public members
/// None.
///
/// # Private members
/// - `by_id`: Indexed canonical branch records.
/// - `inferred_children`: Parent-derived fallback child order.
/// - `visited`: Branches already emitted from this session.
/// - `rows`: Output rows receiving projected content.
/// - `tool_invocation_details`: Safe tool detail output map.
/// - `expanded`: Completed child branches whose activity is visible.
struct BranchTreeState<'a> {
    /// Indexed canonical branch records.
    by_id: &'a HashMap<EntityId, &'a Branch>,
    /// Parent-derived fallback child order.
    inferred_children: &'a HashMap<EntityId, Vec<EntityId>>,
    /// Branches already emitted from this session.
    visited: &'a mut BTreeSet<EntityId>,
    /// Output rows receiving projected content.
    rows: &'a mut Vec<SessionRow>,
    /// Safe tool detail output map.
    tool_invocation_details: &'a mut std::collections::BTreeMap<EntityId, String>,
    /// Completed child branches whose activity is visible.
    expanded: &'a BTreeSet<EntityId>,
}

impl BranchTreeState<'_> {
    /// Appends one branch subtree using an explicit preorder work stack.
    ///
    /// # Parameters
    /// - `self`: Indexed branch state and projection sinks.
    /// - `branch_id`: Initial branch identity for this subtree.
    /// - `depth`: Initial row depth.
    fn append_branch_tree(&mut self, branch_id: EntityId, depth: usize) {
        let mut pending = vec![(branch_id, depth)];
        while let Some((current_id, current_depth)) = pending.pop() {
            if !self.visited.insert(current_id) {
                continue;
            }
            let Some(source) = self.by_id.get(&current_id).copied() else {
                continue;
            };
            let transient = source.status != BranchStatus::Completed;
            self.rows.push(SessionRow {
                reference: source.id,
                depth: current_depth,
                label: format!("{} [{:?}]", source.title, source.status),
                kind: SessionRowKind::Branch,
                candidate: None,
                snapshot: None,
                transient,
            });
            let compact = source.parent.is_some()
                && source.status == BranchStatus::Completed
                && !self.expanded.contains(&source.id);
            if !compact
                && !super::session_activity::append(
                    source,
                    current_depth,
                    transient,
                    self.rows,
                    self.tool_invocation_details,
                )
            {
                super::session_turns::append(
                    source,
                    current_depth,
                    transient,
                    self.rows,
                    self.tool_invocation_details,
                );
            }
            append_result(source, current_depth, self.rows);

            let children = if source.children.is_empty() {
                self.inferred_children.get(&source.id).map(Vec::as_slice)
            } else {
                Some(source.children.as_slice())
            };
            if let Some(children) = children {
                pending.extend(
                    children
                        .iter()
                        .rev()
                        .map(|child| (*child, current_depth.saturating_add(1))),
                );
            }
        }
    }
}

/// Appends a branch's result references and review candidates.
///
/// - `source`: Branch whose durable result is projected.
/// - `depth`: Branch row depth.
/// - `rows`: Output rows receiving result data.
fn append_result(source: &Branch, depth: usize, rows: &mut Vec<SessionRow>) {
    let Some(result) = source.result.as_ref() else {
        return;
    };
    rows.push(SessionRow {
        reference: result.id,
        depth: depth.saturating_add(1),
        label: result
            .summary
            .as_deref()
            .map_or_else(|| "Durable branch result".to_owned(), str::to_owned),
        kind: SessionRowKind::Result,
        candidate: None,
        snapshot: None,
        transient: false,
    });
    rows.extend(result.references.iter().map(|reference| SessionRow {
        reference: *reference,
        depth: depth.saturating_add(2),
        label: format!("Result reference {reference}"),
        kind: SessionRowKind::Result,
        candidate: None,
        snapshot: None,
        transient: false,
    }));
    let base = result
        .result_handles
        .iter()
        .find_map(|handle| handle.strip_prefix("snapshot:"))
        .and_then(|snapshot| snapshot.parse().ok());
    rows.extend(result.result_handles.iter().filter_map(|handle| {
        let candidate = handle.strip_prefix("candidate:")?;
        Some(SessionRow {
            reference: result.id,
            depth: depth.saturating_add(2),
            label: format!("Review candidate {candidate}"),
            kind: SessionRowKind::Candidate,
            candidate: Some(candidate.to_owned()),
            snapshot: base,
            transient: false,
        })
    }));
}
