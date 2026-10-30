//! Compares semantic CAD drawing reads across embedded and protocol surfaces.

use scriberig_app::ApplicationContext;
use scriberig_client::Client;
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_server::InProcessTransport;

use super::super::TestApplication;

type RemoteClient = Client<InProcessTransport<TestApplication>>;
type EmbeddedService = scriberig_app::ApplicationService<
    scriberig_app::StoreInspectionBackend<std::sync::Arc<scriberig_store_sqlite::SqliteStore>>,
>;

/// Confirms exact CAD drawing identities, layers, and entities match remotely.
///
/// - `embedded`: Local application operation boundary.
/// - `remote`: Reusable client over the server protocol.
/// - `context`: Exact-snapshot Inspect authority.
/// - `project`: Project being inspected.
/// - `snapshot`: Exact imported snapshot.
/// - `drawing`: Stable semantic CAD drawing identity.
pub(super) fn assert_cad_reads(
    embedded: &EmbeddedService,
    remote: &RemoteClient,
    context: &ApplicationContext,
    project: ProjectId,
    snapshot: SnapshotId,
    drawing: EntityId,
) {
    let local = embedded
        .inspect_cad_drawing(context, project, snapshot, drawing)
        .expect("embedded CAD summary");
    let remote_summary = remote
        .inspect_cad_drawing(
            project.to_string(),
            snapshot.to_string(),
            drawing.to_string(),
        )
        .expect("remote CAD summary");
    assert_eq!(remote_summary.project, project.to_string());
    assert_eq!(remote_summary.snapshot, snapshot.to_string());
    assert_eq!(remote_summary.drawing, drawing.to_string());
    assert_eq!(remote_summary.entity_count, local.entity_count);
    assert_eq!(
        remote_summary
            .layers
            .iter()
            .map(|layer| (layer.id.clone(), layer.name.clone()))
            .collect::<Vec<_>>(),
        local
            .layers
            .iter()
            .map(|layer| (layer.id.to_string(), layer.name.clone()))
            .collect::<Vec<_>>()
    );
    let local_page = embedded
        .read_cad_entities(context, project, snapshot, drawing, 0, 20)
        .expect("embedded CAD entities");
    let remote_page = remote
        .cad_entities(
            project.to_string(),
            snapshot.to_string(),
            drawing.to_string(),
            0,
            20,
        )
        .expect("remote CAD entities");
    assert_eq!(remote_page.total, local_page.total);
    assert_eq!(
        remote_page
            .entities
            .iter()
            .map(|entity| (entity.id.clone(), entity.layer.clone()))
            .collect::<Vec<_>>(),
        local_page
            .entities
            .iter()
            .map(|entity| (entity.id.to_string(), entity.layer.to_string()))
            .collect::<Vec<_>>()
    );
    assert_cad_text_find(embedded, remote, context, project, snapshot, drawing);
}

/// Confirms CAD text search preserves stable entity handles across the protocol.
///
/// - `embedded`: Local application operation boundary.
/// - `remote`: Reusable client over the server protocol.
/// - `context`: Exact-snapshot Inspect authority.
/// - `project`: Project being inspected.
/// - `snapshot`: Exact imported snapshot.
/// - `drawing`: Stable semantic CAD drawing identity.
fn assert_cad_text_find(
    embedded: &EmbeddedService,
    remote: &RemoteClient,
    context: &ApplicationContext,
    project: ProjectId,
    snapshot: SnapshotId,
    drawing: EntityId,
) {
    let local = embedded
        .find_cad_text(
            context,
            &scriberig_app::CadTextSearchRequest {
                project,
                snapshot,
                drawing,
                query: "parcel 124".into(),
                offset: 0,
                limit: 10,
            },
        )
        .expect("embedded CAD text search");
    let remote = remote
        .find_cad_text(
            project.to_string(),
            snapshot.to_string(),
            drawing.to_string(),
            "parcel 124",
            0,
            10,
        )
        .expect("remote CAD text search");
    assert_eq!(remote.project, project.to_string());
    assert_eq!(remote.snapshot, snapshot.to_string());
    assert_eq!(remote.drawing, drawing.to_string());
    assert_eq!(remote.total, local.total);
    assert_eq!(remote.matches.len(), 1);
    assert_eq!(
        remote.matches[0].entity.id,
        local.matches[0].entity.id.to_string()
    );
    assert_eq!(remote.matches[0].text, "Parcel 124");
}
