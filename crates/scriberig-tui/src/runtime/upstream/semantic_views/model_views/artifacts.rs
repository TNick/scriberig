//! Renders bounded artifact provenance and fidelity in the RPI Markdown panel.

use super::super::common::{
    MAX_ROWS, SemanticRole, append_status, append_value, render_load_state,
};
use crate::{
    inspector::LoadState,
    view::{ArtifactMembersModel, artifacts::ArtifactPageModel},
};

/// Renders one bounded artifact page and its selected safety details.
///
/// # Parameters
/// - `state`: Artifact request state supplied by the shared TUI reducer.
/// - `members_state`: Exact member-page request state supplied by the reducer.
///
/// # Returns
/// Markdown content for the upstream terminal component.
pub(super) fn render(
    state: &LoadState<ArtifactPageModel>,
    members_state: &LoadState<ArtifactMembersModel>,
) -> String {
    let mut content = render_load_state("Project artifacts", state, |model, output| {
        append_value(output, &format!("Project: {}", model.project));
        append_value(
            output,
            &format!("{} of {} artifacts", model.artifacts.len(), model.total),
        );
        for (index, artifact) in model.artifacts.iter().take(MAX_ROWS).enumerate() {
            render_artifact(model, artifact, output);
            if index + 1 < model.artifacts.len() {
                append_value(output, "---");
            }
        }
        if let Some(offset) = model.next_offset {
            append_value(
                output,
                &format!("Next page: /artifacts {} {offset}", model.project),
            );
        }
    });
    render_member_page(members_state, &mut content);
    content
}

/// Appends one artifact's bounded provenance and fidelity details.
///
/// # Parameters
/// - `model`: Page containing the artifact and project identity.
/// - `artifact`: Artifact row to render.
/// - `output`: Markdown buffer receiving the artifact details.
fn render_artifact(
    model: &ArtifactPageModel,
    artifact: &crate::view::artifacts::ArtifactRow,
    output: &mut String,
) {
    append_value(
        output,
        &format!("## {} · {}", artifact.logical_key, artifact.kind),
    );
    append_value(output, &format!("Artifact: {}", artifact.id));
    append_value(output, &format!("Source adapter: {}", artifact.adapter));
    append_value(output, &format!("Scan digest: {}", artifact.scan_id));
    let synchronization = format!(
        "{} · baseline {}{}",
        artifact.synchronization_status,
        if artifact.baseline_digest.is_empty() {
            "unavailable"
        } else {
            &artifact.baseline_digest
        },
        artifact
            .observed_digest
            .as_ref()
            .map(|digest| format!(" · observed {digest}"))
            .unwrap_or_default()
    );
    if artifact.synchronization_status == "in-sync" {
        append_value(
            output,
            &format!("Source synchronization: {synchronization}"),
        );
    } else {
        append_status(
            output,
            "SOURCE OUT OF SYNC",
            &synchronization,
            SemanticRole::Warning,
        );
    }
    let fidelity = format!(
        "{} · opaque content {}",
        artifact.fidelity, artifact.opaque_preservation
    );
    if is_restricted(&artifact.fidelity) {
        append_status(
            output,
            "EDITING RESTRICTED",
            &fidelity,
            SemanticRole::Warning,
        );
    } else {
        append_value(output, &format!("Fidelity: {fidelity}"));
    }
    if let Some(diagnostic) = &artifact.diagnostic {
        append_status(
            output,
            "Capability reason",
            diagnostic,
            SemanticRole::Warning,
        );
    }
    append_value(output, &format!("Members ({})", artifact.member_count));
    for member in artifact.members.iter().take(8) {
        let location = member
            .project_path
            .as_deref()
            .or(member.container_part.as_deref())
            .unwrap_or("logical member");
        append_value(
            output,
            &format!("  {} · {} · {}", member.key, location, member.digest),
        );
    }
    if artifact.member_count > artifact.members.len().min(8) {
        append_value(
            output,
            &format!(
                "Read members: /artifact-members {} {} 0",
                model.project, artifact.id
            ),
        );
    }
    append_value(
        output,
        &format!("References ({})", artifact.references.len()),
    );
    for reference in artifact.references.iter().take(8) {
        append_value(output, reference);
    }
    if artifact.references.len() > 8 {
        append_value(output, "Additional references are omitted from this view.");
    }
}

/// Appends one bounded member page and its continuation command.
///
/// # Parameters
/// - `state`: Exact member page load state.
/// - `content`: Existing artifact panel content to extend.
fn render_member_page(state: &LoadState<ArtifactMembersModel>, content: &mut String) {
    if !matches!(state, LoadState::NotRequested) {
        content.push_str("\n\n");
        content.push_str(&render_load_state(
            "Artifact members",
            state,
            |page, output| {
                append_value(
                    output,
                    &format!("{} of {} members", page.members.len(), page.total),
                );
                for member in &page.members {
                    let location = member
                        .project_path
                        .as_deref()
                        .or(member.container_part.as_deref())
                        .unwrap_or("logical member");
                    append_value(
                        output,
                        &format!("{} · {} · {}", member.key, location, member.digest),
                    );
                }
                if let Some(offset) = page.next_offset {
                    append_value(
                        output,
                        &format!(
                            "Next page: /artifact-members {} {} {offset}",
                            page.project, page.artifact
                        ),
                    );
                }
            },
        ));
    }
}

/// Classifies artifact fidelity levels whose edits need prominent restriction details.
///
/// # Parameters
/// - `fidelity`: Shared artifact fidelity label.
///
/// # Returns
/// True when edits are unavailable or constrained by provider/preservation requirements.
fn is_restricted(fidelity: &str) -> bool {
    matches!(
        fidelity,
        "unsupported" | "inspect-only" | "read-only-derived" | "restricted-external-provider-edit"
    )
}
