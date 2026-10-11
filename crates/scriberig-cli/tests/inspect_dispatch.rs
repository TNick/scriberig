//! Verifies CLI inspect dispatch through the application query boundary.

use scriberig_app::{
    ApplicationContext, ApplicationError, DetailLevel, InspectRequest, InspectResult, QueryService,
    SnapshotResolution,
};
use scriberig_cli::{
    Command, InspectArgs, dispatch,
    trust::{InspectDepth, OutputFormat},
};
use scriberig_core::{EntityId, RevisionId, SnapshotId};

/// Supplies a deterministic application inspection result.
struct Fixture;

impl QueryService for Fixture {
    /// Returns the requested entity under the requested snapshot.
    ///
    /// # Parameters
    /// - `self`: Fixture service.
    /// - `_context`: Application request context.
    /// - `request`: Snapshot-aware inspection request.
    ///
    /// # Returns
    /// Deterministic inspection result.
    ///
    /// # Errors
    /// Never returns an error.
    fn inspect(
        &self,
        _context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        assert_eq!(
            request.snapshot,
            SnapshotResolution::At(SnapshotId::new([3; 32]))
        );
        assert_eq!(request.detail, DetailLevel::Compact);
        Ok(InspectResult {
            entity: request.entity,
            revision: Some(RevisionId::new([2; 32])),
            snapshot: Some(SnapshotId::new([3; 32])),
            object_type: None,
            label: Some("fixture".to_owned()),
            description: None,
            capabilities: Vec::new(),
            trust: None,
        })
    }
}

/// Verifies optional snapshot parsing is preserved by CLI dispatch.
#[test]
fn inspect_dispatch_preserves_handles() {
    let entity = EntityId::new([1; 16]);
    let result = dispatch(
        &Fixture,
        &ApplicationContext::empty(),
        Command::Inspect(InspectArgs {
            entity,
            snapshot: Some(SnapshotId::new([3; 32])),
            depth: InspectDepth::Compact,
            format: OutputFormat::Human,
        }),
    )
    .expect("dispatch")
    .expect("output");
    assert!(result.contains(&entity.to_string()));
    assert!(result.contains(&SnapshotId::new([3; 32]).to_string()));
    assert!(result.contains(&RevisionId::new([2; 32]).to_string()));
}

/// Verifies unsupported command paths return a non-zero dispatch status.
#[test]
fn unsupported_dispatch_is_structured() {
    let error = dispatch(&Fixture, &ApplicationContext::empty(), Command::Maintenance)
        .expect_err("unsupported command");
    assert_eq!(error.exit_code, 1);
}
