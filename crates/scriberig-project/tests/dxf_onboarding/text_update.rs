//! Verifies exact-base semantic DXF text candidate editing.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use scriberig_core::{EntityId, ProjectId, RevisionId, SnapshotId};
use scriberig_domain_cad::{CadEntity, DrawingEntity};
use scriberig_project::{CandidateValidation, CandidateValidationProfile, ProjectService};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

/// Holds the exact imported project evidence for one text-edit test.
struct TextFixture {
    /// Temporary project checkout and source file.
    root: PathBuf,
    /// Temporary durable project database path.
    database: PathBuf,
    /// Shared store retaining the exact base snapshot.
    store: Arc<SqliteStore>,
    /// Project operation service used by the test.
    service: ProjectService,
    /// Exact project identity.
    project: ProjectId,
    /// Immutable imported source snapshot.
    base: SnapshotId,
    /// Stable CAD drawing entity.
    drawing: EntityId,
    /// Stable target text entity.
    target: EntityId,
    /// Exact target revision at the base snapshot.
    target_revision: RevisionId,
    /// Original semantic entity used to verify preserved properties.
    original: DrawingEntity,
    /// Untouched neighboring entity.
    neighbor: EntityId,
    /// Exact neighbor revision at the base snapshot.
    neighbor_revision: RevisionId,
    /// Exact source bytes captured before candidate creation.
    source: Vec<u8>,
}

/// Builds a project with an imported label and an unrelated CAD entity.
///
/// # Returns
/// Temporary project fixture with exact source and semantic target evidence.
fn prepare_text_fixture() -> TextFixture {
    let root = super::dxf_project();
    let source_path = root.join("roads.dxf");
    let mut drawing = dxf::Drawing::load(&mut std::fs::File::open(&source_path).expect("source"))
        .expect("parse source DXF");
    let mut label =
        dxf::entities::Entity::new(dxf::entities::EntityType::Text(dxf::entities::Text {
            location: dxf::Point::new(1.0, 2.0, 3.0),
            value: "Parcel 124".to_owned(),
            text_height: 2.5,
            ..Default::default()
        }));
    "LABELS".clone_into(&mut label.common.layer);
    drawing.add_entity(label);
    let mut source = Vec::new();
    drawing.save(&mut source).expect("serialize source DXF");
    fs::write(&source_path, &source).expect("save label fixture");

    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open project store"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan project").project.id;
    let base = service.import(project).expect("import DXF").snapshot;
    let owned = store.project_entities(project).expect("owned entities");
    let drawing = owned
        .iter()
        .find(|record| record.kind == "cad.drawing")
        .expect("drawing identity")
        .entity;
    let target_record = owned
        .iter()
        .filter(|record| record.kind == "cad.entity")
        .find(|record| {
            let revision = store
                .resolve(base, record.entity)
                .expect("resolve entity")
                .expect("entity revision");
            let payload = store
                .get(revision)
                .expect("load entity")
                .expect("entity payload");
            let semantic: DrawingEntity =
                serde_json::from_slice(payload.payload()).expect("decode entity");
            matches!(&semantic.geometry, CadEntity::Text { value, .. } if value == "Parcel 124")
        })
        .expect("parcel label");
    let target = target_record.entity;
    let original_revision = store
        .resolve(base, target)
        .expect("resolve target")
        .expect("target revision");
    let original = serde_json::from_slice(
        store
            .get(original_revision)
            .expect("load target")
            .expect("target payload")
            .payload(),
    )
    .expect("decode target");
    let neighbor = owned
        .iter()
        .find(|record| record.kind == "cad.entity" && record.entity != target)
        .expect("neighbor entity")
        .entity;
    let neighbor_revision = store
        .resolve(base, neighbor)
        .expect("resolve neighbor")
        .expect("neighbor revision");
    TextFixture {
        root,
        database,
        store,
        service,
        project,
        base,
        drawing,
        target,
        target_revision: original_revision,
        original,
        neighbor,
        neighbor_revision,
        source,
    }
}

/// Edits one text value through candidate validation and shared materialization.
#[test]
fn text_update_candidate_preserves_target_properties_and_other_entities() {
    let fixture = prepare_text_fixture();
    let source_path = fixture.root.join("roads.dxf");
    let candidate = fixture
        .service
        .propose_dxf_text_update(fixture.project, fixture.base, fixture.target, "Parcel 125")
        .expect("propose text update");
    assert_eq!(
        candidate.semantic_operation.as_deref(),
        Some("cad.update_text")
    );
    assert_eq!(candidate.semantic_changes[0].target, fixture.target);
    assert_eq!(candidate.semantic_changes[0].property, "entity.text");
    assert_eq!(
        candidate.semantic_changes[0].before,
        serde_json::json!("Parcel 124")
    );
    assert_eq!(
        candidate.semantic_changes[0].after,
        serde_json::json!("Parcel 125")
    );
    assert_eq!(
        fixture
            .service
            .candidate(&candidate.id)
            .expect("reload candidate")
            .expect("candidate")
            .semantic_changes,
        candidate.semantic_changes
    );
    assert_eq!(
        fs::read(&source_path).expect("live bytes before apply"),
        fixture.source
    );
    let validated = fixture
        .service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::DxfPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate text candidate");
    assert_eq!(validated.validation, CandidateValidation::Passed);
    let applied = fixture
        .service
        .apply_candidate(&validated)
        .expect("apply text candidate");
    let changed = fixture
        .service
        .cad_entities(
            fixture.project,
            applied.successor_snapshot,
            fixture.drawing,
            0,
            20,
        )
        .expect("read successor entities");
    let edited = changed
        .entities
        .iter()
        .find(|entity| entity.id == fixture.target)
        .expect("same label identity");
    let CadEntity::Text {
        position: old_position,
        height: old_height,
        ..
    } = fixture.original.geometry
    else {
        panic!("target must be text");
    };
    assert_eq!(
        edited.geometry,
        CadEntity::Text {
            position: old_position,
            value: "Parcel 125".to_owned(),
            height: old_height,
        }
    );
    assert_eq!(
        fixture
            .store
            .resolve(applied.successor_snapshot, fixture.neighbor)
            .expect("resolve unchanged neighbor"),
        Some(fixture.neighbor_revision)
    );
    assert_ne!(
        fixture
            .store
            .resolve(applied.successor_snapshot, fixture.target)
            .expect("resolve edited target"),
        Some(fixture.target_revision)
    );
    assert_ne!(
        fs::read(&source_path).expect("materialized bytes"),
        fixture.source
    );

    drop(fixture.service);
    drop(fixture.store);
    fs::remove_file(fixture.database).expect("remove project store");
    fs::remove_dir_all(fixture.root).expect("remove fixture project");
}
