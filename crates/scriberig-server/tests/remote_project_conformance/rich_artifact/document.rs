//! Compares semantic document reads across embedded and protocol surfaces.

use scriberig_app::{ApplicationContext, DocumentBlockView, DocumentParagraphSearchRequest};
use scriberig_client::Client;
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_protocol::DocumentBlockView as ProtocolDocumentBlockView;
use scriberig_server::InProcessTransport;

use super::super::TestApplication;

type RemoteClient = Client<InProcessTransport<TestApplication>>;
type EmbeddedService = scriberig_app::ApplicationService<
    scriberig_app::StoreInspectionBackend<std::sync::Arc<scriberig_store_sqlite::SqliteStore>>,
>;

/// Confirms exact document identities and paragraph content match remotely.
///
/// - `embedded`: Local application operation boundary.
/// - `remote`: Reusable client over the server protocol.
/// - `context`: Exact-snapshot Inspect authority.
/// - `project`: Project being inspected.
/// - `snapshot`: Exact imported snapshot.
/// - `document`: Stable semantic document identity.
pub(super) fn assert_document_reads(
    embedded: &EmbeddedService,
    remote: &RemoteClient,
    context: &ApplicationContext,
    project: ProjectId,
    snapshot: SnapshotId,
    document: EntityId,
) {
    let local = embedded
        .inspect_document(context, project, snapshot, document)
        .expect("embedded document tree");
    let remote_tree = remote
        .inspect_document(
            project.to_string(),
            snapshot.to_string(),
            document.to_string(),
        )
        .expect("remote document tree");
    assert_eq!(remote_tree.project, project.to_string());
    assert_eq!(remote_tree.snapshot, snapshot.to_string());
    assert_eq!(remote_tree.document, document.to_string());
    assert_eq!(
        remote_tree
            .sections
            .iter()
            .map(|section| section.id.clone())
            .collect::<Vec<_>>(),
        local
            .sections
            .iter()
            .map(|section| section.id.to_string())
            .collect::<Vec<_>>()
    );
    let local_paragraphs = local
        .sections
        .iter()
        .flat_map(|section| &section.blocks)
        .filter_map(|block| match block {
            DocumentBlockView::Paragraph { paragraph } => Some((
                paragraph.id.to_string(),
                paragraph
                    .runs
                    .iter()
                    .map(|run| run.text.as_str())
                    .collect::<String>(),
            )),
            DocumentBlockView::Table { .. } => None,
        })
        .collect::<Vec<_>>();
    let remote_paragraphs = remote_tree
        .sections
        .iter()
        .flat_map(|section| &section.blocks)
        .filter_map(|block| match block {
            ProtocolDocumentBlockView::Paragraph { paragraph } => Some((
                paragraph.id.clone(),
                paragraph
                    .runs
                    .iter()
                    .map(|run| run.text.as_str())
                    .collect::<String>(),
            )),
            ProtocolDocumentBlockView::Table { .. } => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(remote_paragraphs, local_paragraphs);
    assert_document_search(embedded, remote, context, project, snapshot, document);
}

/// Confirms paragraph search returns the same stable handle and text remotely.
///
/// - `embedded`: Local application operation boundary.
/// - `remote`: Reusable client over the server protocol.
/// - `context`: Exact-snapshot Inspect authority.
/// - `project`: Project being inspected.
/// - `snapshot`: Exact imported snapshot.
/// - `document`: Stable semantic document identity.
fn assert_document_search(
    embedded: &EmbeddedService,
    remote: &RemoteClient,
    context: &ApplicationContext,
    project: ProjectId,
    snapshot: SnapshotId,
    document: EntityId,
) {
    let local = embedded
        .find_document_paragraphs(
            context,
            &DocumentParagraphSearchRequest {
                project,
                snapshot,
                document,
                query: "site".into(),
                offset: 0,
                limit: 10,
            },
        )
        .expect("embedded paragraph search");
    let remote = remote
        .find_document_paragraphs(
            project.to_string(),
            snapshot.to_string(),
            document.to_string(),
            "site",
            0,
            10,
        )
        .expect("remote paragraph search");
    assert_eq!(remote.project, project.to_string());
    assert_eq!(remote.snapshot, snapshot.to_string());
    assert_eq!(remote.document, document.to_string());
    assert_eq!(remote.total, local.matches.len());
    assert_eq!(remote.matches.len(), local.matches.len());
    assert_eq!(remote.matches.len(), 1);
    assert_eq!(
        remote.matches[0].paragraph,
        local.matches[0].paragraph.to_string()
    );
    assert_eq!(
        remote.matches[0].section,
        local.matches[0].section.to_string()
    );
    assert_eq!(remote.matches[0].text, "Existing site area");
}
