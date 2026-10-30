//! Formats `ScribeRig` semantic and projection read models.

use super::common::{MAX_ROWS, append_load_notice, append_value, optional_id, render_load_state};
use crate::inspector::LoadState;
use crate::state::TuiState;

/// Renders the selected exact semantic entity summary.
///
/// - `state`: Current inspector and selection projections.
///
/// # Returns
/// Overview content or the inspector request state.
pub(super) fn overview(state: &TuiState) -> String {
    render_load_state("Overview", &state.inspector, |model, output| {
        append_value(output, &format!("Entity: {}", model.entity));
        if let Some(revision) = model.revision {
            append_value(output, &format!("Revision: {revision}"));
        }
        if let Some(snapshot) = model.snapshot {
            append_value(output, &format!("Snapshot: {snapshot}"));
        }
        if let Some(kind) = &model.kind {
            append_value(output, &format!("Kind: {kind}"));
        }
        if let Some(label) = &model.label {
            append_value(output, &format!("# {label}"));
        }
        if let Some(description) = &model.description {
            append_value(output, description);
        }
        super::common::append_values(output, "Capabilities", &model.capabilities);
    })
}

/// Renders plan-view guidance when no durable plan tree is available.
///
/// - `state`: Active session snapshot and plan selection.
///
/// Safe Markdown describing the plan view's current availability.
/// # Returns
/// Guidance text for the active plan view.
pub(super) fn plan(state: &TuiState) -> String {
    if state.session_snapshot.is_none() {
        return "# Plans\n\nLoad a session to inspect its durable plans.".to_owned();
    }
    "# Plans\n\nNo durable plans exist in this session. Use the existing `/plan` command in the composer to create one.".to_owned()
}

/// Renders known semantic explorer references without inferring file trees.
///
/// - `state`: Explicit explorer root and result rows.
///
/// # Returns
/// Snapshot-bound explorer content.
pub(super) fn explorer(state: &TuiState) -> String {
    let mut output = String::from("# Semantic explorer\n\n");
    if let Some(root) = &state.explorer.root {
        append_value(
            &mut output,
            &format!(
                "{}Root: {}",
                if state.cursor == 0 { "› " } else { "  " },
                root.selection.entity
            ),
        );
        if let Some(snapshot) = root.selection.snapshot {
            append_value(&mut output, &format!("Snapshot: {snapshot}"));
        }
    }
    for (index, row) in state.explorer.rows.iter().take(MAX_ROWS).enumerate() {
        append_value(
            &mut output,
            &format!(
                "{}{} · {} · {}",
                if index.saturating_add(1) == state.cursor {
                    "› "
                } else {
                    "  "
                },
                row.label,
                row.kind.as_deref().unwrap_or("semantic entity"),
                row.selection.entity
            ),
        );
        if let Some(snapshot) = row.selection.snapshot {
            append_value(&mut output, &format!("  snapshot {snapshot}"));
        }
    }
    if state.explorer.next.is_some() {
        append_value(&mut output, "More semantic results are available.");
    }
    output
}

/// Renders exact migration history and result-snapshot identities.
///
/// - `state`: History load state and bounded migration projection.
///
/// # Returns
/// History content or its loading/error state.
pub(super) fn history(state: &TuiState) -> String {
    render_load_state("History", &state.history, |model, output| {
        append_value(output, &format!("Snapshot: {}", model.snapshot));
        for (index, row) in model.rows.iter().take(MAX_ROWS).enumerate() {
            append_value(
                output,
                &format!(
                    "{}{} · {} → {}",
                    if index == state.cursor { "› " } else { "  " },
                    row.id,
                    row.base,
                    row.result
                ),
            );
            for (entity, revision) in row.entities.iter().take(32) {
                append_value(
                    output,
                    &format!("  {entity} · revision {}", optional_id(*revision)),
                );
            }
        }
    })
}

/// Renders bounded semantic discovery results.
///
/// - `state`: Find request state and stable result references.
///
/// # Returns
/// Find content or its loading/error state.
pub(super) fn find(state: &TuiState) -> String {
    render_load_state("Find", &state.find, |model, output| {
        append_value(output, &format!("Query: {}", model.query));
        for (index, row) in model.rows.iter().take(MAX_ROWS).enumerate() {
            append_value(
                output,
                &format!(
                    "{}{} · {} · snapshot {}",
                    if index == state.cursor { "› " } else { "  " },
                    row.entity,
                    row.entity_type.as_deref().unwrap_or("entity"),
                    row.snapshot
                ),
            );
            if let Some(preview) = &row.preview {
                append_value(output, preview);
            }
        }
    })
}

/// Renders structured semantic deltas with both exact snapshot identities.
///
/// - `state`: Diff request state and entity revision transitions.
///
/// # Returns
/// Diff content or its loading/error state.
pub(super) fn diff(state: &TuiState) -> String {
    render_load_state("Semantic diff", &state.diff, |model, output| {
        append_value(output, &format!("Base snapshot: {}", model.left));
        append_value(output, &format!("Target snapshot: {}", model.right));
        for (index, row) in model.rows.iter().take(MAX_ROWS).enumerate() {
            append_value(
                output,
                &format!(
                    "{}{} · {} → {}",
                    if index == state.cursor { "› " } else { "  " },
                    row.entity,
                    optional_id(row.before),
                    optional_id(row.after)
                ),
            );
        }
    })
}

/// Renders a bounded read-only projection and its semantic source reference.
///
/// - `state`: Projection load state and safe rendered content.
///
/// # Returns
/// Projection content or its loading/error state.
pub(super) fn projection(state: &TuiState) -> String {
    render_load_state("Projection", &state.projection, |model, output| {
        if let Some(projection) = &model.projection {
            append_value(output, &format!("Projection: {projection}"));
        }
        append_value(output, &format!("Source entity: {}", model.source_entity));
        if let Some(path) = &model.source_path {
            append_value(output, &format!("Source path: {path}"));
        }
        if let Some(revision) = model.source_revision {
            append_value(output, &format!("Source revision: {revision}"));
        }
        append_value(output, &format!("Base snapshot: {}", model.base_snapshot));
        append_value(
            output,
            &format!("Renderer: {} · style {}", model.renderer, model.style),
        );
        if let Some(synchronization) = &model.synchronization {
            append_value(output, &format!("Synchronization: {synchronization}"));
        }
        for line in model.content.lines().take(MAX_ROWS) {
            append_value(output, line);
        }
    })
}

/// Renders candidate, conflict, and validation evidence without promotion.
///
/// - `state`: Current review model and retained failed proposal.
///
/// # Returns
/// Candidate review content or the review request state.
pub(super) fn changes(state: &TuiState) -> String {
    let model = match &state.review {
        LoadState::Ready(model) => Some(model),
        _ => state.review_proposal.as_ref(),
    };
    let mut output = String::from("# Candidate review\n\n");
    let Some(model) = model else {
        return append_load_notice(output, "Review", &state.review);
    };
    append_value(&mut output, &format!("Change: {}", model.change));
    append_value(
        &mut output,
        &format!("Base snapshot: {}", model.base_snapshot),
    );
    if let Some(snapshot) = model.candidate_snapshot {
        append_value(&mut output, &format!("Result snapshot: {snapshot}"));
    } else {
        super::common::append_status(
            &mut output,
            "Result snapshot",
            "unavailable; promotion is blocked.",
            super::common::SemanticRole::Warning,
        );
    }
    let validation_role = match &model.validation {
        crate::view::changes::ReviewValidation::Passed => super::common::SemanticRole::Success,
        crate::view::changes::ReviewValidation::Failed(_) => super::common::SemanticRole::Error,
        crate::view::changes::ReviewValidation::Unknown => super::common::SemanticRole::Warning,
    };
    super::common::append_status(
        &mut output,
        "Validation",
        &format!("{:?}", model.validation),
        validation_role,
    );
    if let Some(profile) = &model.validation_profile {
        append_value(&mut output, &format!("Validation profile: {profile}"));
    }
    for (index, delta) in model.deltas.iter().take(MAX_ROWS).enumerate() {
        append_value(
            &mut output,
            &format!(
                "Revision change {}: entity {} · {} → {}",
                index + 1,
                delta.entity,
                optional_id(delta.before),
                optional_id(delta.after)
            ),
        );
    }
    for entity in model.affected_entities.iter().take(MAX_ROWS) {
        if !model.deltas.iter().any(|delta| delta.entity == *entity) {
            append_value(&mut output, &format!("Affected entity: {entity}"));
        }
    }
    for conflict in model.conflicts.iter().take(MAX_ROWS) {
        super::common::append_status(
            &mut output,
            "Conflict",
            conflict,
            super::common::SemanticRole::Warning,
        );
    }
    for detail in model.details.iter().take(MAX_ROWS) {
        append_value(&mut output, detail);
    }
    if model.requires_confirmation {
        if model.candidate_snapshot.is_none() {
            append_value(
                &mut output,
                "Confirmation is unavailable without the exact result snapshot.",
            );
        }
        super::common::append_status(
            &mut output,
            "Confirmation",
            "required for this candidate.",
            super::common::SemanticRole::Warning,
        );
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{changes, projection};
    use crate::inspector::LoadState;
    use crate::state::TuiState;
    use crate::view::changes::{ChangeReviewModel, ReviewValidation};
    use scriberig_core::{EntityId, RevisionId, SnapshotId};

    /// Keeps exact candidate identity and evidence visible in review Markdown.
    #[test]
    fn changes_projection_shows_snapshots_validation_and_revision_evidence() {
        let base = SnapshotId::new([1; 32]);
        let result = SnapshotId::new([2; 32]);
        let entity = EntityId::from_bytes([3; 16]);
        let before = RevisionId::new([4; 32]);
        let after = RevisionId::new([5; 32]);
        let review = ChangeReviewModel {
            base_snapshot: base,
            candidate_snapshot: Some(result),
            change: "candidate-7".to_owned(),
            branch: None,
            affected_entities: vec![entity],
            deltas: vec![
                sr_change::EntityDelta::new(entity, Some(before), Some(after))
                    .expect("revision delta"),
            ],
            details: vec!["materialized semantic projection".to_owned()],
            conflicts: Vec::new(),
            validation: ReviewValidation::Passed,
            validation_profile: Some("strict-policy".to_owned()),
            requires_confirmation: true,
        };
        let state = TuiState {
            review: LoadState::Ready(review),
            ..TuiState::default()
        };

        let output = changes(&state);

        assert!(output.contains(&format!("Base snapshot: {base}")));
        assert!(output.contains(&format!("Result snapshot: {result}")));
        assert!(output.contains("Validation profile: strict-policy"));
        assert!(output.contains(&format!("entity {entity}")));
        assert!(output.contains(&before.to_string()));
        assert!(output.contains(&after.to_string()));
        assert!(output.contains("materialized semantic projection"));
    }

    /// Explains why review cannot be confirmed without a result snapshot.
    #[test]
    fn changes_projection_blocks_confirmation_without_result_identity() {
        let state = TuiState {
            review: LoadState::Ready(ChangeReviewModel {
                base_snapshot: SnapshotId::new([6; 32]),
                candidate_snapshot: None,
                change: "candidate-no-result".to_owned(),
                branch: None,
                affected_entities: Vec::new(),
                deltas: Vec::new(),
                details: Vec::new(),
                conflicts: Vec::new(),
                validation: ReviewValidation::Passed,
                validation_profile: None,
                requires_confirmation: true,
            }),
            ..TuiState::default()
        };

        assert!(changes(&state).contains("Confirmation is unavailable"));
    }

    /// Retains exact projection identity without inventing synchronization state.
    #[test]
    fn projection_shows_exact_metadata_and_safe_content() {
        let entity = EntityId::from_bytes([7; 16]);
        let snapshot = SnapshotId::new([8; 32]);
        let state = TuiState {
            projection: LoadState::Ready(crate::view::projection::ProjectionModel {
                projection: Some("projection-1".to_owned()),
                source_entity: entity,
                source_path: Some("src/main.rs".to_owned()),
                source_revision: None,
                style: "markdown".to_owned(),
                renderer: "renderer-v1".to_owned(),
                content: "safe content".to_owned(),
                synchronization: None,
                base_snapshot: snapshot,
            }),
            ..TuiState::default()
        };

        let output = projection(&state);

        assert!(output.contains("projection-1"));
        assert!(output.contains(&entity.to_string()));
        assert!(output.contains("src/main.rs"));
        assert!(output.contains(&snapshot.to_string()));
        assert!(output.contains("safe content"));
        assert!(!output.contains("synchronized"));
    }
}
