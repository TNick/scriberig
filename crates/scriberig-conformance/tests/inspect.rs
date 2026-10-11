//! Compares inspect semantics across direct, remote, CLI, and agent surfaces.

use scriberig_agent_core::{Tool, ToolDetail, ToolInput};
use scriberig_app::{
    ApplicationContext, DetailLevel, InspectRequest, QueryService, SnapshotResolution,
};
use scriberig_cli::{
    Command, InspectArgs, dispatch,
    trust::{InspectDepth, OutputFormat},
};
use scriberig_client::Client;
use scriberig_conformance::{allowed_context, fixture, inspect_tool};
use scriberig_protocol::{SnapshotReference, SnapshotSelector};
use scriberig_server::InProcessTransport;

/// Verifies all implemented inspect surfaces retain semantic handles.
#[test]
fn inspect_surfaces_are_semantically_equal() {
    let expected_fixture = fixture();
    let expected = expected_fixture
        .inspect(
            &allowed_context(),
            &InspectRequest {
                entity: expected_fixture.entity,
                snapshot: SnapshotResolution::At(expected_fixture.snapshot),
                detail: DetailLevel::Compact,
            },
        )
        .expect("direct inspect");
    let remote = Client::new(InProcessTransport::new(fixture()))
        .inspect(
            expected.entity.to_string(),
            SnapshotSelector::Exact(SnapshotReference {
                id: expected.snapshot.expect("snapshot").to_string(),
            }),
            false,
        )
        .expect("remote inspect");
    assert_eq!(remote.entity.id, expected.entity.to_string());
    assert_eq!(
        remote.revision.id,
        expected.revision.expect("revision").to_string()
    );
    assert_eq!(
        remote.snapshot.id,
        expected.snapshot.expect("snapshot").to_string()
    );

    let agent = inspect_tool()
        .invoke(
            &allowed_context(),
            ToolInput {
                entity: expected.entity,
                snapshot: Some(expected.snapshot.expect("snapshot")),
                detail: ToolDetail::Compact,
            },
        )
        .expect("agent inspect");
    assert_eq!(agent.entity, expected.entity);
    assert_eq!(agent.revision, expected.revision);
    assert_eq!(agent.snapshot, expected.snapshot);

    let cli = dispatch(
        &fixture(),
        &ApplicationContext::empty(),
        Command::Inspect(InspectArgs {
            entity: expected.entity,
            snapshot: expected.snapshot,
            depth: InspectDepth::Compact,
            format: OutputFormat::Json,
        }),
    )
    .expect("CLI inspect")
    .expect("CLI output");
    assert!(cli.contains(&expected.entity.to_string()));
    assert!(cli.contains(&expected.revision.expect("revision").to_string()));
}

/// Verifies missing entities retain a structured category across surfaces.
#[test]
fn missing_entity_is_structured() {
    let fixture = fixture();
    let error = fixture
        .inspect(
            &ApplicationContext::empty(),
            &InspectRequest {
                entity: scriberig_core::EntityId::new([7; 16]),
                snapshot: SnapshotResolution::Current,
                detail: DetailLevel::Compact,
            },
        )
        .expect_err("missing entity");
    assert_eq!(error.category(), "not_found");
}
