//! Compares inspect semantics across direct, remote, CLI, and agent surfaces.

use scriberig_agent_core::{Tool, ToolDetail, ToolInput};
use scriberig_app::{
    ApplicationContext, DetailLevel, InspectRequest, QueryService, SnapshotResolution,
};
use scriberig_cli::{
    InspectArgs,
    inspect::result,
    trust::{InspectDepth, OutputFormat},
};
use scriberig_client::Client;
use scriberig_conformance::{allowed_context, fixture, inspect_tool};
use scriberig_protocol::{SnapshotReference, SnapshotSelector};
use scriberig_server::InProcessTransport;
use scriberig_tui::inspect_headless;

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

    let cli = result(
        &fixture(),
        &ApplicationContext::empty(),
        &InspectArgs {
            entity: expected.entity,
            snapshot: expected.snapshot,
            depth: InspectDepth::Compact,
            format: OutputFormat::Json,
            db: "playground/dev-app-runtime/scriberig.sqlite".into(),
        },
    )
    .expect("CLI inspect");
    assert_eq!(cli.entity, expected.entity);
    assert_eq!(cli.revision, expected.revision);
    assert_eq!(cli.snapshot, expected.snapshot);
    assert_eq!(cli.label, expected.label);
    assert_eq!(cli.description, expected.description);

    let tui = inspect_headless(
        &expected_fixture,
        &allowed_context(),
        expected.entity,
        expected.snapshot,
    )
    .expect("TUI inspect");
    assert_eq!(tui.entity, expected.entity);
    assert_eq!(tui.revision, expected.revision);
    assert_eq!(tui.snapshot, expected.snapshot);
}

/// Verifies runtime capability discovery survives the server/client boundary.
#[test]
fn runtime_capabilities_are_consistent_across_surfaces() {
    let fixture = fixture();
    let direct = scriberig_server::ApplicationService::capabilities(&fixture);
    let remote = Client::new(InProcessTransport::new(fixture)).capabilities();
    let remote = remote.expect("runtime capabilities");
    assert_eq!(remote.capabilities.len(), direct.len());
    for (remote, direct) in remote.capabilities.iter().zip(direct.iter()) {
        assert_eq!(remote.id, direct.id.to_string());
        assert_eq!(remote.description, direct.description);
        assert_eq!(remote.available, direct.available);
    }
}

/// Verifies missing entities retain a structured category across surfaces.
#[test]
fn missing_entity_is_structured() {
    let fixture = fixture();
    let error = fixture
        .inspect(
            &ApplicationContext::empty(),
            &InspectRequest {
                entity: scriberig_core::EntityId::from_bytes([7; 16]),
                snapshot: SnapshotResolution::Current,
                detail: DetailLevel::Compact,
            },
        )
        .expect_err("missing entity");
    assert_eq!(error.category(), "not_found");
}
