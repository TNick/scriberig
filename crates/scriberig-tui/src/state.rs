//! Stores presentation-only state for the initial terminal shell.
#[path = "state_behavior.rs"]
mod behavior;

use std::collections::{BTreeSet, HashSet};

use crate::{
    action::Action,
    effect::Effect,
    explorer::ExplorerModel,
    inspector::{InspectorModel, LoadState},
    navigation::NavigationState,
    overlay::{Overlay, input::InputState},
    selection::EntitySelection,
};
use scriberig_app::RuntimeCapability;

#[path = "state_presentation_types.rs"]
mod presentation_types;
pub use presentation_types::{
    ConnectionState, ConversationComposerState, ConversationTarget, PromptVisibility,
    ReasoningVisibility, SessionComposerState, ToolInvocationVisibility, ToolOutputVisibility,
    TransientAgentActivity, TransientAgentOutput,
};
pub(crate) use presentation_types::{PresentationControl, sanitize_paste};

/// Retains one stable selected node while its model assignment picker is open.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlanAssignmentTarget {
    /// Durable plan identity.
    pub plan: scriberig_core::EntityId,
    /// Durable plan-node identity.
    pub node: scriberig_core::EntityId,
    /// Plan revision observed before opening the picker.
    pub expected_revision: u64,
}

/// Runtime and presentation state for the initial TUI shell.
#[allow(clippy::struct_excessive_bools)] // Independent UI toggles are persisted presentation preferences.
#[derive(Clone, Debug, PartialEq)]
pub struct TuiState {
    /// Whether the event loop should terminate.
    pub should_quit: bool,
    /// A short status message rendered in the shell.
    pub shell_message: String,
    /// Sanitized multiline output from the latest `/recover` inspection.
    pub recovery_message: Option<String>,
    /// Stable active project identity, when selected.
    pub project: Option<String>,
    /// Current semantic-service connection state.
    pub connection: ConnectionState,
    /// Whether cached models require authoritative refresh after reconnect.
    pub stale: bool,
    /// Semantic navigation history, independent from pane focus.
    pub navigation: NavigationState,
    /// The active modal overlay, if one is open.
    pub overlay: Option<Overlay>,
    /// Presentation cursor index.
    pub cursor: usize,
    /// Stable semantic selection independent of the visible cursor.
    pub selection: Option<EntitySelection>,
    /// Stable identity of the selected flat semantic-view row.
    pub semantic_row_cursor: Option<crate::selection::SemanticRowIdentity>,
    /// Stable plan/node navigation, independent of the conversation and coordinator cursors.
    pub plan_cursor: Option<crate::selection::PlanSelection>,
    /// Exact project scope available to the first agent conversation.
    pub agent_selection: Option<scriberig_app::AgentSelection>,
    /// Semantic explorer model.
    pub explorer: ExplorerModel,
    /// Latest bounded logical artifact page for the terminal details view.
    pub artifact_page: LoadState<crate::view::artifacts::ArtifactPageModel>,
    /// Latest bounded exact member page requested for one artifact.
    pub artifact_members: LoadState<crate::view::ArtifactMembersModel>,
    /// Latest exact-snapshot CAD drawing and layer summary.
    pub cad_drawing: LoadState<scriberig_protocol::CadDrawingInspectResponse>,
    /// Latest bounded exact-snapshot CAD entity page.
    pub cad_entities: LoadState<scriberig_protocol::CadEntitiesResponse>,
    /// Latest CAD layer-change candidate awaiting shared review.
    pub cad_layer_edit: LoadState<scriberig_protocol::CadLayerChangeResponse>,
    /// Zero-based offset of the currently loaded CAD entity page.
    pub cad_offset: usize,
    /// Latest bounded exact-snapshot worksheet page.
    pub spreadsheet_page: LoadState<scriberig_protocol::SpreadsheetCellsResponse>,
    /// Latest exact-coordinate spreadsheet inspection.
    pub spreadsheet_cell: LoadState<scriberig_protocol::SpreadsheetCellInspectResponse>,
    /// Worksheet retained while the semantic selection points at one of its cells.
    pub spreadsheet_worksheet: Option<scriberig_core::EntityId>,
    /// Latest spreadsheet edit candidate awaiting shared change review.
    pub spreadsheet_edit: LoadState<scriberig_app::SpreadsheetEditCandidate>,
    /// Latest exact-snapshot document tree for TUI outline and detail views.
    pub document_tree: LoadState<scriberig_protocol::DocumentInspectResponse>,
    /// Root identity retained while a paragraph or table cell is selected.
    pub document_root: Option<scriberig_core::EntityId>,
    /// Snapshot owning `document_root` and the loaded document tree.
    pub document_snapshot: Option<scriberig_core::SnapshotId>,
    /// Zero-based offset into the flattened bounded document outline.
    pub document_offset: usize,
    /// Latest document paragraph candidate awaiting shared review.
    pub document_edit: LoadState<scriberig_app::DocumentParagraphCandidate>,
    /// Latest document table-cell candidate awaiting shared change review.
    pub document_cell_edit: LoadState<scriberig_app::DocumentParagraphCandidate>,
    /// Latest bounded `GeoPackage` layer catalog.
    pub geo_catalog: LoadState<scriberig_protocol::GeoCatalogResponse>,
    /// Latest exact-snapshot QGIS project and its layer-reference navigation links.
    pub qgis_project: LoadState<scriberig_protocol::QgisProjectResponse>,
    /// Latest QGIS layer rename candidate awaiting shared review.
    pub qgis_layer_rename: LoadState<scriberig_protocol::QgisLayerRenameResponse>,
    /// Latest QGIS project layer-order candidate awaiting shared review.
    pub qgis_layer_order: LoadState<scriberig_protocol::QgisLayerOrderResponse>,
    /// Latest personal source projection-style preference update.
    pub projection_style_preference: LoadState<scriberig_protocol::ProjectionStyleSettingResponse>,
    /// Latest exact effective source style, including source chain and fidelity.
    pub projection_style_resolution: LoadState<scriberig_protocol::ProjectionStyleResolveResponse>,
    /// Latest bounded `GeoPackage` feature page.
    pub geo_features: LoadState<scriberig_protocol::GeoFeaturesResponse>,
    /// Latest bounded geospatial query result with exact source provenance.
    pub geo_query_result: LoadState<scriberig_protocol::GeoFeatureQueryResponse>,
    /// Latest geospatial feature attribute candidate awaiting shared review.
    pub geo_edit: LoadState<scriberig_app::GeoAttributeCandidate>,
    /// Latest geospatial feature geometry candidate awaiting shared review.
    pub geo_geometry_edit: LoadState<scriberig_app::GeoGeometryCandidate>,
    /// Exact artifact path owning the currently loaded geospatial catalog.
    pub geo_path: Option<String>,
    /// Exact immutable snapshot owning the currently loaded geospatial catalog.
    pub geo_snapshot: Option<scriberig_core::SnapshotId>,
    /// Current inspector request state.
    pub inspector: LoadState<InspectorModel>,
    /// Runtime capabilities supplied by the application adapter.
    pub capabilities: Vec<RuntimeCapability>,
    /// Next request identifier.
    pub next_request_id: u64,
    /// Monotonic presentation-state revision for cached upstream projections.
    pub render_revision: u64,
    /// Selected primary view.
    pub primary_view: crate::selection::PrimaryView,
    /// History request/result state.
    pub history: LoadState<crate::view::history::HistoryModel>,
    /// Find request/result state.
    pub find: LoadState<crate::view::find::FindModel>,
    /// Diff request/result state.
    pub diff: LoadState<crate::view::diff::DiffModel>,
    /// Projection request/result state.
    pub projection: LoadState<crate::view::projection::ProjectionModel>,
    /// Change review request/result state.
    pub review: LoadState<crate::view::changes::ChangeReviewModel>,
    /// Candidate handle and exact base snapshot the current review generation was
    /// requested from. Kept while that generation is outstanding or settled so a
    /// superseding session refresh can reuse it instead of orphaning its response.
    pub review_request: Option<scriberig_app::ApplyRequest>,
    /// Last operation failure retained for review and refresh feedback.
    pub operation_error: Option<(String, String)>,
    /// Last reviewed proposal retained after a failed confirmation.
    pub review_proposal: Option<crate::view::changes::ChangeReviewModel>,
    /// Bounded runtime activity retained independently of local visibility.
    pub transient_agent_activities: Vec<TransientAgentActivity>,
    /// Operation observations retained for the operations view.
    pub operations: Vec<crate::view::operations::OperationModel>,
    /// Session tree request/result state.
    pub session: LoadState<crate::view::session::SessionModel>,
    /// Stable selected session row independent of its flattened position.
    pub session_cursor: Option<crate::view::session::SessionRowIdentity>,
    /// Conversation projection rebuilt from the latest durable session snapshot.
    pub conversation: Option<crate::view::conversation::ConversationModel>,
    /// Latest shared session snapshot used to rebuild local projections.
    pub session_snapshot: Option<scriberig_session::Session>,
    /// Conversation composer targeting an exact semantic location.
    pub conversation_composer: ConversationComposerState,
    /// Stable conversation cursor independent of rendered row index.
    pub conversation_cursor: Option<crate::view::conversation::ConversationSource>,
    /// Whether the user is browsing rows instead of editing the draft.
    pub conversation_browsing: bool,
    /// Durable node currently followed by the coordinator cursor.
    pub coordinator_cursor: Option<(scriberig_core::EntityId, scriberig_core::EntityId)>,
    /// Whether the conversation should follow the plan coordinator.
    pub follow_coordinator: bool,
    /// Client-local branch expansion identities.
    pub expanded_branches: BTreeSet<scriberig_core::EntityId>,
    /// Credential-safe model library available to the interactive picker.
    pub model_library: LoadState<scriberig_protocol::ModelLibraryResponse>,
    /// Whether a pending model-library read was opened from thinking controls.
    /// Selected plan node awaiting an assignment choice.
    pub plan_assignment_target: Option<PlanAssignmentTarget>,
    /// Branch to reveal after the matching agent submission finishes.
    pub session_branch_focus: Option<scriberig_core::EntityId>,
    /// Operation identities with an authoritative refresh already requested.
    pub pending_operation_refresh: Vec<scriberig_core::OperationId>,
    /// Bounded streamed assistant output keyed by shared operation identity.
    pub transient_agent_outputs: Vec<TransientAgentOutput>,
    /// Next local sequence for transient rows received from the shared stream.
    pub next_transient_sequence: u64,
    /// Stable session, branch, and operation handles for active agent work.
    pub agent_handles: Vec<crate::service::AgentHandle>,
    /// Persistent session composer draft, when an agent session is active.
    pub session_composer: Option<SessionComposerState>,
    /// Whether the persistent composer owns keyboard input.
    pub session_composer_focused: bool,
    /// Client-local tool-result verbosity.
    pub tool_output_visibility: ToolOutputVisibility,
    /// Client-local tool-call presentation mode.
    pub tool_invocation_visibility: ToolInvocationVisibility,
    /// Client-local prompt/model-request visibility.
    pub prompt_visibility: PromptVisibility,
    /// Client-local reasoning-summary visibility.
    pub reasoning_visibility: ReasoningVisibility,
    /// Shared preference revision applied to the TUI presentation layer.
    pub presentation_preferences_revision: Option<u64>,
    /// Presentation controls whose shared defaults are locally overridden.
    pub(crate) presentation_overrides: HashSet<PresentationControl>,
    /// Captured safe request detail indexed by model invocation identity.
    pub model_request_details: std::collections::BTreeMap<
        scriberig_core::EntityId,
        scriberig_protocol::ModelRequestDetailResponse,
    >,
    /// Expanded safe detail cached by durable tool-result identity.
    pub tool_result_details: std::collections::BTreeMap<
        scriberig_core::EntityId,
        scriberig_protocol::ToolResultDetailResponse,
    >,
}

pub(crate) use behavior::{
    conversation_projection, output_is_durable, refresh_conversation_projection,
};
#[path = "state_actions.rs"]
mod state_actions;
#[path = "state_cursor_reducer.rs"]
mod state_cursor_reducer;
#[path = "state_model_selection.rs"]
mod state_model_selection;
#[path = "state_presentation.rs"]
mod state_presentation;
#[path = "state_reducer.rs"]
mod state_reducer;
#[path = "state_reducer_tail.rs"]
mod state_reducer_tail;
pub(crate) use state_actions::activate_palette_item;
use state_actions::reduce_application_event;
pub(crate) use state_model_selection::{select_model, select_thinking_level};
pub(super) use state_presentation::activate_palette_control;
pub use state_reducer::reduce;

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
