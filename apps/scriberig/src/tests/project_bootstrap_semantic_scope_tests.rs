//! Verifies exact-snapshot semantic discovery in project bootstrap workflows.

use super::*;

/// Discovers architecture files by semantic source path and exact snapshot.
///
/// - `tools`: Shared project semantic application tool adapter.
/// - `context`: Trusted exact-snapshot context for the imported project.
/// - `project`: Stable imported project identity.
/// - `snapshot`: Exact immutable project snapshot.
/// - `super`: Parent test module containing the bootstrap scenario.
///
/// # Panics
/// Panics when a representative architecture source cannot be found or rendered.
pub(super) fn assert_semantic_architecture(
    tools: &LocalAgentTool<'_, impl scriberig_app::ApplicationBackend>,
    context: &ApplicationContext,
    project: ProjectId,
    snapshot: SnapshotId,
) {
    for path in [
        "Cargo.toml",
        "capabilities.toml",
        "crates/domains/scriberig-domain-code/src/lib.rs",
        "crates/domains/scriberig-domain-python/src/lib.rs",
        "crates/domains/test-support/src/conformance.rs",
        "crates/scriberig-project/src/adapters.rs",
        "crates/scriberig-project/src/scan.rs",
    ] {
        let found = tools
            .invoke_named(
                context,
                "semantic_find",
                &serde_json::json!({
                    "project": project.to_string(),
                    "snapshot": snapshot.to_string(),
                    "query": path,
                    "limit": 50
                }),
            )
            .expect("semantic architecture find");
        let source = found
            .handles
            .iter()
            .filter_map(|handle| serde_json::from_str::<serde_json::Value>(handle).ok())
            .find(|value| {
                value.get("path").and_then(serde_json::Value::as_str) == Some(path)
                    && value.get("kind").and_then(serde_json::Value::as_str) == Some("source")
            })
            .unwrap_or_else(|| panic!("exact semantic architecture handle for {path}: {found:?}"));
        assert_eq!(source["project"], project.to_string());
        assert_eq!(source["snapshot"], snapshot.to_string());
        let entity = source["entity"].as_str().expect("semantic source entity");
        let rendered = tools
            .invoke_named(
                context,
                "semantic_render",
                &serde_json::json!({
                    "project": project.to_string(),
                    "snapshot": snapshot.to_string(),
                    "entity": entity
                }),
            )
            .expect("semantic architecture render");
        assert!(
            rendered
                .handles
                .iter()
                .any(|handle| { handle == &format!("snapshot:{snapshot}") })
        );
        assert!(
            rendered
                .handles
                .iter()
                .any(|handle| handle.starts_with("source_digest:"))
        );
        assert!(
            rendered
                .summary
                .as_deref()
                .is_some_and(|text| text.contains(path))
        );
    }
}

/// Proves semantic references remain tied to their immutable discovery snapshot.
///
/// - `tools`: Shared project semantic application tool adapter.
/// - `base_context`: Trusted scope compiled for the original snapshot.
/// - `store`: Durable source and snapshot store used by project service.
/// - `project`: Stable imported project identity.
/// - `base`: Exact snapshot that owns the original references.
/// - `root`: Live checkout rescanned to create a successor snapshot.
/// - `super`: Parent test module containing bootstrap scenario assertions.
///
/// # Panics
/// Panics when the old context is rebased or the successor context accepts stale refs.
pub(super) fn assert_stale_discovery_scope(
    tools: &LocalAgentTool<'_, impl scriberig_app::ApplicationBackend>,
    base_context: &ApplicationContext,
    store: &Arc<SqliteStore>,
    project: ProjectId,
    base: SnapshotId,
    root: &std::path::Path,
) {
    fs::write(
        root.join("AGENTS.md"),
        "# Updated guidance\nUse the successor snapshot only.\n",
    )
    .expect("update semantic guidance fixture");
    let project_service = scriberig_project::ProjectService::new(Arc::clone(store));
    let scan = project_service
        .scan(None, root)
        .expect("scan successor project state");
    assert_eq!(scan.project.id, project);
    let manifest = project_service
        .manifest_summary(project)
        .expect("successor manifest summary");
    let successor = project_service
        .import_manifest(project, &manifest.manifest_id)
        .expect("import successor project state")
        .snapshot;
    assert_ne!(successor, base);

    let successor_context = base_context.clone().with_snapshot(successor);
    let old_context_reference = tools.invoke_named(
        base_context,
        "semantic_find",
        &serde_json::json!({
            "project": project.to_string(),
            "snapshot": base.to_string(),
            "query": "AGENTS.md",
            "limit": 10
        }),
    );
    assert!(matches!(
        old_context_reference,
        Err(scriberig_agent_core::ToolError::Application { category, .. })
            if category == "stale_snapshot"
    ));

    let fresh_find = tools
        .invoke_named(
            &successor_context,
            "semantic_find",
            &serde_json::json!({
                "project": project.to_string(),
                "snapshot": successor.to_string(),
                "query": "AGENTS.md",
                "limit": 10
            }),
        )
        .expect("successor context inspects successor snapshot");
    assert_eq!(fresh_find.snapshot, Some(successor));
    let guidance = fresh_find
        .handles
        .iter()
        .find(|value| value.contains("AGENTS.md"))
        .expect("successor guidance handle");
    let entity: EntityId = serde_json::from_str::<serde_json::Value>(guidance)
        .expect("semantic handle")
        .get("entity")
        .and_then(serde_json::Value::as_str)
        .expect("semantic entity")
        .parse()
        .expect("entity identity");
    let fresh_render = tools
        .invoke_named(
            &successor_context,
            "semantic_render",
            &serde_json::json!({
                "project": project.to_string(),
                "snapshot": successor.to_string(),
                "entity": entity.to_string()
            }),
        )
        .expect("successor context renders successor source");
    assert_eq!(fresh_render.snapshot, Some(successor));
    assert!(
        fresh_render
            .summary
            .as_deref()
            .is_some_and(|value| value.contains("Use the successor snapshot only"))
    );

    let cross_snapshot_reference = tools.invoke_named(
        &successor_context,
        "semantic_render",
        &serde_json::json!({
            "project": project.to_string(),
            "snapshot": base.to_string(),
            "entity": entity.to_string()
        }),
    );
    assert!(matches!(
        cross_snapshot_reference,
        Err(scriberig_agent_core::ToolError::CapabilityDenied { .. })
    ));
}
