//! Verifies rich-project relations survive an unrelated domain candidate edit.

use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;

use scriberig_app::{ApplicationContext, CommitRequest, OperationProvider, StoreOperationProvider};
use scriberig_core::{EntityId, ObjectType, RevisionId, RootName, SnapshotId};
use scriberig_domain_geo::GeoValue;
use scriberig_project::{
    CandidateValidation, CandidateValidationProfile, GeoPackageFeatureAttributeEdit, ProjectService,
};
use scriberig_store::{Snapshot, SnapshotRelation, SnapshotRelationChange, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;
use sr_change::{Migration, Operation, Provenance, SourceKind};

/// Verifies a document-to-feature relation and unrelated entity revisions survive reimport.
#[test]
fn document_to_geo_relation_survives_feature_edit_and_project_reimport() {
    let (_temporary, fixture) = copy_mixed_fixture();
    let database = fixture.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open SQLite store"));
    let projects = ProjectService::new(Arc::clone(&store));
    let project = projects
        .scan(None, &fixture)
        .expect("scan fixture")
        .project
        .id;
    let initial = projects.import(project).expect("import fixture").snapshot;
    let entities = store.project_entities(project).expect("project entities");
    let paragraph = entity(&entities, "document.paragraph", "report.docx");
    let layer = entity(&entities, "geo.layer", "parcels.gpkg");
    let feature = projects
        .geopackage_feature_page(project, "parcels.gpkg", initial, layer, 0, 2)
        .expect("read exact feature page")
        .features
        .into_iter()
        .next()
        .expect("imported feature")
        .id;
    let unrelated_cell = entity(&entities, "spreadsheet.cell", "budget.xlsx");
    let paragraph_revision = revision(&store, initial, paragraph);
    let feature_revision = revision(&store, initial, feature);
    let cell_revision = revision(&store, initial, unrelated_cell);
    let relation = SnapshotRelation::new(
        paragraph,
        ObjectType::new("document.references").expect("relation type"),
        feature,
    );
    let linked = commit_relation(
        Arc::clone(&store),
        project,
        initial,
        paragraph,
        feature,
        relation.clone(),
    );
    let feature_data = projects
        .geopackage_feature_page(project, "parcels.gpkg", linked, layer, 0, 2)
        .expect("read feature after relation commit")
        .features
        .into_iter()
        .find(|item| item.id == feature)
        .expect("linked feature");
    let candidate = projects
        .propose_geopackage_feature_attribute(&GeoPackageFeatureAttributeEdit {
            project,
            base_snapshot: linked,
            path: "parcels.gpkg".into(),
            layer,
            feature,
            field: "area".into(),
            value: GeoValue::Number("15.5".into()),
        })
        .expect("propose feature edit");
    let validated = projects
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::GeoPackagePackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate feature edit");
    assert_eq!(validated.validation, CandidateValidation::Passed);
    assert!(feature_data.attributes.contains_key("area"));
    let applied = projects
        .apply_candidate(&validated)
        .expect("apply feature edit");

    assert_ne!(applied.successor_snapshot, linked);
    assert_eq!(
        revision(&store, applied.successor_snapshot, paragraph),
        paragraph_revision
    );
    assert_ne!(
        revision(&store, applied.successor_snapshot, feature),
        feature_revision
    );
    assert_eq!(
        revision(&store, applied.successor_snapshot, unrelated_cell),
        cell_revision
    );
    assert_eq!(
        store.relations(applied.successor_snapshot),
        Ok(vec![relation])
    );
    assert_eq!(
        projects
            .workspace(project)
            .expect("current workspace")
            .snapshot,
        applied.successor_snapshot
    );
}

/// Copies the checked-in mixed rich-artifact fixture into an isolated project directory.
///
/// # Returns
/// The copied project directory.
fn copy_mixed_fixture() -> (tempfile::TempDir, std::path::PathBuf) {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let project = temporary.path().join("project");
    fs::create_dir_all(&project).expect("create project directory");
    let source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/rich-project");
    for entry in fs::read_dir(source).expect("read fixture directory") {
        let entry = entry.expect("fixture entry");
        if entry.file_type().expect("fixture type").is_file() {
            fs::copy(entry.path(), project.join(entry.file_name())).expect("copy fixture member");
        }
    }
    (temporary, project)
}

/// Finds one project entity by semantic kind and source path.
///
/// # Parameters
/// - `entities`: Imported project ownership records.
/// - `kind`: Exact semantic entity kind.
/// - `path`: Project-relative source path.
///
/// # Returns
/// The matching entity identity.
fn entity(
    entities: &[scriberig_store_sqlite::ProjectEntityRecord],
    kind: &str,
    path: &str,
) -> EntityId {
    entities
        .iter()
        .find(|item| item.kind == kind && item.relative_path == path)
        .unwrap_or_else(|| panic!("missing {kind} in {path}"))
        .entity
}

/// Resolves one exact revision from an immutable snapshot.
///
/// # Parameters
/// - `store`: Snapshot and revision store.
/// - `snapshot`: Exact semantic snapshot.
/// - `entity`: Semantic entity identity.
///
/// # Returns
/// The exact immutable revision.
fn revision(store: &SqliteStore, snapshot: SnapshotId, entity: EntityId) -> RevisionId {
    store
        .resolve(snapshot, entity)
        .expect("resolve entity revision")
        .expect("entity exists in snapshot")
}

/// Commits a declared semantic relation through the normal migration provider.
///
/// # Parameters
/// - `store`: Shared project store.
/// - `project`: Project owning the relation endpoint entities.
/// - `base`: Imported project snapshot.
/// - `from`: Referencing document paragraph.
/// - `to`: Referenced geospatial feature.
/// - `relation`: Typed relation to add.
///
/// # Returns
/// The snapshot produced by committing the relation migration.
fn commit_relation(
    store: Arc<SqliteStore>,
    project: scriberig_core::ProjectId,
    base: SnapshotId,
    from: EntityId,
    to: EntityId,
    relation: SnapshotRelation,
) -> SnapshotId {
    let entities = store.project_entities(project).expect("project entities");
    let entries = entities
        .iter()
        .map(|item| (item.entity, revision(&store, base, item.entity)))
        .collect::<BTreeMap<_, _>>();
    let expected = Snapshot::from_entries(entries)
        .apply_relation_changes([SnapshotRelationChange::Add(relation)])
        .id();
    let migration = Migration::new(
        base,
        expected,
        Vec::new(),
        vec![Operation::AddRelation {
            from,
            relation: "document.references".into(),
            to,
        }],
        Provenance {
            source: SourceKind::User,
            actor: None,
            workspace: None,
            projection: None,
            validation: Vec::new(),
            parents: Vec::new(),
            metadata: BTreeMap::new(),
        },
    )
    .expect("relation migration");
    let provider = StoreOperationProvider::new(store);
    assert!(provider.register_migration("cross-domain-reference", migration));
    provider
        .commit(
            &ApplicationContext::empty(),
            &CommitRequest {
                base_snapshot: base,
                branch: Some(RootName::new(format!("project_{project}")).expect("project root")),
                change: "cross-domain-reference".into(),
            },
        )
        .expect("commit relation migration")
        .snapshot
        .expect("committed snapshot")
}
