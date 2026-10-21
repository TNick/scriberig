//! Holds borrowed state for one composer input operation.

use super::{
    Arc, Component, ComposerInput, ControlKind, Editor, Effect, Mutex, TuiAltScreen, TuiDebugLog,
    TuiState, Work,
};

/// Borrows runtime state required to process one composer input event.
///
/// # Public members
/// None.
///
/// # Private members
/// - `service`: Application operations for model properties.
/// - `context`: Authorization context for application operations.
/// - `state`: Shared `ScribeRig` presentation state.
/// - `screen`: Upstream terminal screen.
/// - `editor`: Upstream composer editor.
/// - `composer`: `ScribeRig` composer adapter.
/// - `semantic_view`: `ScribeRig` semantic navigation adapter.
/// - `input_sender`: Channel for follow-up composer inputs.
/// - `work_sender`: Worker command channel.
/// - `pending_selector`: Model selector waiting for a model library response.
/// - `pending_effects`: Effects waiting for worker capacity.
/// - `focus_before_overlay`: Focus target to restore after closing overlays.
/// - `shared_state`: Shared state used by upstream selectors.
/// - `debug_log`: Optional redacted input diagnostics.
/// - `should_quit`: Event loop termination flag.
/// - `redraw`: Whether the upstream screen needs a render request.
pub struct InputContext<'a, 'log, S> {
    /// Application operations for model properties.
    pub service: &'a S,
    /// Authorization context for application operations.
    pub context: &'a scriberig_app::ApplicationContext,
    /// Shared `ScribeRig` presentation state.
    pub state: &'a Arc<Mutex<TuiState>>,
    /// Upstream terminal screen.
    pub screen: &'a Arc<TuiAltScreen>,
    /// Upstream composer editor.
    pub editor: &'a Editor,
    /// `ScribeRig` composer adapter.
    pub composer: &'a Arc<super::super::super::components::ComposerAdapter>,
    /// `ScribeRig` semantic navigation adapter.
    pub semantic_view: &'a Arc<super::super::super::semantic_views::SemanticViewPanel>,
    /// Channel for follow-up composer inputs.
    pub input_sender: &'a std::sync::mpsc::Sender<ComposerInput>,
    /// Worker command channel.
    pub work_sender: &'a std::sync::mpsc::SyncSender<Work>,
    /// Model selector waiting for a model library response.
    pub pending_selector: &'a mut Option<ControlKind>,
    /// Effects waiting for worker capacity.
    pub pending_effects: &'a mut std::collections::VecDeque<Effect>,
    /// Focus target to restore after closing overlays.
    pub focus_before_overlay: &'a mut Option<Arc<dyn Component>>,
    /// Shared state used by upstream selectors.
    pub shared_state: &'a Arc<Mutex<TuiState>>,
    /// Optional redacted input diagnostics.
    pub debug_log: &'a mut Option<&'log mut TuiDebugLog>,
    /// Event loop termination flag.
    pub should_quit: &'a mut bool,
    /// Whether the upstream screen needs a render request.
    pub redraw: &'a mut bool,
}
