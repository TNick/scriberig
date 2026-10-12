//! Provides the application-backed terminal interaction boundary and shell.

use std::io;

use scriberig_app::{ApplicationContext, ApplicationError, InspectResult};
use scriberig_core::{EntityId, SnapshotId};

pub mod action;
pub mod effect;
pub mod event;
pub mod explorer;
pub mod inspector;
pub mod keymap;
pub mod layout;
pub mod navigation;
pub mod overlay;
pub mod render;
pub mod runtime;
pub mod selection;
pub mod service;
pub mod state;
pub mod terminal;
pub mod view;

/// Configures the first application-backed TUI shell.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TuiConfig {
    /// Runs a line-oriented interactive shell when true.
    pub interactive: bool,
    /// Optional entity inspected when the shell starts.
    pub initial_entity: Option<EntityId>,
    /// Optional exact snapshot for the initial inspection.
    pub snapshot: Option<SnapshotId>,
    /// Optional active project identity shown in the shell context.
    pub project: Option<String>,
}

/// Describes why the TUI runtime stopped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunOutcome {
    /// The user requested normal termination.
    Quit,
}

/// Structured TUI runtime failure.
#[derive(Debug)]
pub enum TuiError {
    /// The application returned a structured semantic failure.
    Application(ApplicationError),
    /// Terminal input or output failed.
    Io(io::Error),
    /// A command line contained invalid semantic input.
    InvalidCommand(String),
}

impl std::fmt::Display for TuiError {
    /// Formats a safe terminal runtime error.
    /// - `formatter`: Argument named `formatter`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Application(error) => error.fmt(formatter),
            Self::Io(error) => error.fmt(formatter),
            Self::InvalidCommand(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for TuiError {}

impl From<io::Error> for TuiError {
    /// Converts terminal I/O failures into the TUI error model.
    /// - `error`: Argument named `error`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Provides semantic operations required by the TUI shell.
pub use service::{
    ApplicationAdapter, ApplicationOperationAdapter, ExplorerLabels, RemoteAdapter,
    TuiOperationService, TuiService,
};

/// Runs the line-oriented TUI shell over an application service.
///
/// # Parameters
/// - `service`: Application service used for semantic inspection.
/// - `context`: Caller capability and snapshot context.
/// - `config`: TUI interaction and initial-selection configuration.
///
/// # Returns
/// The terminal session outcome.
///
/// # Errors
/// Returns terminal I/O, invalid-command, or application failures.
pub fn run<S: TuiService + Sync>(
    service: &S,
    context: &ApplicationContext,
    config: &TuiConfig,
) -> Result<RunOutcome, TuiError> {
    runtime::run(service, context, config).map_err(TuiError::Io)?;
    Ok(RunOutcome::Quit)
}

/// Executes one inspection without taking over the caller's terminal.
///
/// # Parameters
/// - `service`: Application service used for semantic inspection.
/// - `context`: Caller capability and snapshot context.
/// - `entity`: Entity to inspect.
/// - `snapshot`: Optional exact snapshot selector.
///
/// # Returns
/// Snapshot-bound inspection result.
///
/// # Errors
/// Returns an application or TUI boundary failure.
pub fn inspect_headless<S: TuiService>(
    service: &S,
    context: &ApplicationContext,
    entity: EntityId,
    snapshot: Option<SnapshotId>,
) -> Result<InspectResult, TuiError> {
    service
        .inspect_entity(context, entity, snapshot)
        .map_err(TuiError::Application)
}

/// Renders one structured inspection in the shell's compact text form.
#[cfg(test)]
mod tests {
    use super::{RunOutcome, TuiConfig, inspect_headless};
    use scriberig_app::{ApplicationContext, ApplicationError, InspectRequest, InspectResult};
    use scriberig_core::{EntityId, ObjectType, RevisionId, SnapshotId};

    /// Provides a deterministic TUI service for headless tests.
    struct Fixture;

    impl scriberig_app::QueryService for Fixture {
        /// Returns one deterministic inspection.
        /// - `_context`: Argument named `_context`.
        /// - `request`: Argument named `request`.
        ///
        /// # Returns
        /// The value produced by the declaration.
        ///
        /// # Errors
        /// Returns an error when the operation cannot complete.
        fn inspect(
            &self,
            _context: &ApplicationContext,
            request: &InspectRequest,
        ) -> Result<InspectResult, ApplicationError> {
            Ok(InspectResult {
                entity: request.entity,
                revision: Some(RevisionId::new([2; 32])),
                snapshot: Some(SnapshotId::new([3; 32])),
                object_type: Some(ObjectType::new("test.entity").expect("type")),
                label: Some("fixture".to_owned()),
                description: None,
                capabilities: Vec::new(),
                trust: None,
            })
        }
    }

    /// Verifies the TUI service preserves application semantic identity.
    #[test]
    fn headless_inspection_uses_shared_service() {
        let result = inspect_headless(
            &Fixture,
            &ApplicationContext::empty(),
            EntityId::from_bytes([1; 16]),
            None,
        )
        .expect("inspection");
        assert_eq!(result.entity, EntityId::from_bytes([1; 16]));
    }

    /// Verifies noninteractive execution does not own the terminal.
    #[test]
    fn configuration_defaults_are_explicit() {
        assert!(!TuiConfig::default().interactive);
        assert_eq!(RunOutcome::Quit, RunOutcome::Quit);
    }
}
