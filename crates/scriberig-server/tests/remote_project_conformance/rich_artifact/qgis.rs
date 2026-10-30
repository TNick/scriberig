//! Compares exact-snapshot QGIS project reads across app and client APIs.

use scriberig_app::{ApplicationContext, QgisProjectQuery};
use scriberig_client::Client;
use scriberig_core::{ProjectId, SnapshotId};
use scriberig_protocol::QgisProjectRequest;
use scriberig_server::InProcessTransport;

use super::super::TestApplication;

type RemoteClient = Client<InProcessTransport<TestApplication>>;
type EmbeddedService = scriberig_app::ApplicationService<
    scriberig_app::StoreInspectionBackend<std::sync::Arc<scriberig_store_sqlite::SqliteStore>>,
>;

/// Confirms the QGIS Parcels reference resolves to the expected `GeoLayer`.
///
/// - `embedded`: Local application operation boundary.
/// - `remote`: Reusable client over the server protocol.
/// - `context`: Exact-snapshot Inspect authority.
/// - `project`: Project being inspected.
/// - `snapshot`: Exact imported snapshot.
/// - `layer`: Stable semantic `GeoPackage` layer identity.
pub(super) fn assert_qgis_read(
    embedded: &EmbeddedService,
    remote: &RemoteClient,
    context: &ApplicationContext,
    project: ProjectId,
    snapshot: SnapshotId,
    layer: scriberig_core::EntityId,
) {
    let local = embedded
        .inspect_qgis_project(
            context,
            &QgisProjectQuery {
                project,
                path: "project.qgs".into(),
                snapshot,
            },
        )
        .expect("embedded QGIS project");
    let remote = remote
        .inspect_qgis_project(QgisProjectRequest {
            project: project.to_string(),
            path: "project.qgs".into(),
            snapshot: snapshot.to_string(),
        })
        .expect("remote QGIS project");
    assert_eq!(remote.project, project.to_string());
    assert_eq!(remote.snapshot, snapshot.to_string());
    assert_eq!(remote.layers.len(), local.qgis.project.layers.len());
    assert_eq!(
        remote
            .layers
            .iter()
            .find(|reference| reference.name == "Parcels")
            .and_then(|reference| reference.layer.as_deref()),
        Some(layer.to_string().as_str())
    );
}
