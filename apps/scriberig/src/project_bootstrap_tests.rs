//! Verifies project bootstrap through the shared agent tool boundary.

#[cfg(test)]
mod tests {
    use std::fs;
    use std::{process::Command, sync::Arc};

    use scriberig_agent_core::Tool;
    use scriberig_agent_runtime::AgentMessage;
    use scriberig_app::{
        AgentCommandRequest, ApplicationBackendComposition, ApplicationContext, ApplicationService,
        BootstrapEnvironment, Capability, CapabilityAction, CapabilityConstraints,
        CapabilityContext, CapabilityScope, StoreInspectionBackend, StoreOperationProvider,
    };
    use scriberig_core::{EntityId, ProjectId, SnapshotId};
    use scriberig_store_sqlite::SqliteStore;

    use crate::{
        agent_tool::LocalAgentTool, deterministic_provider::BootstrapProvider,
        native_tui_agent::execute_agent,
    };

    /// Scans, reviews, imports, and resolves a project using agent tools only.
    ///
    /// The assertions stay together so the deterministic runtime transcript is checked as one flow.
    #[test]
    #[allow(clippy::too_many_lines)]
    fn bootstrap_tools_use_trusted_path_and_exact_reviewed_manifest() {
        let root =
            std::env::temp_dir().join(format!("scriberig-agent-onboard-{0}", EntityId::new()));
        let database = root.with_extension("sqlite");
        fs::create_dir_all(root.join("src")).expect("fixture root");
        fs::write(root.join("Cargo.toml"), "[package]\nname='fixture'\n").expect("manifest");
        fs::write(root.join("src/lib.rs"), "pub fn value() -> u8 { 1 }\n").expect("source");
        fs::write(
            root.join("AGENTS.md"),
            "# Agent guidance\nUse semantic project references for source discovery.\n",
        )
        .expect("agent guidance");
        add_architecture_fixture(&root);
        for index in 0..101 {
            let path = root.join(format!("docs/page-{index:03}.txt"));
            fs::create_dir_all(path.parent().expect("page parent")).expect("page directory");
            fs::write(path, format!("page {index}\n")).expect("manifest pagination fixture");
        }
        let git = |arguments: &[&str]| {
            assert!(
                Command::new("git")
                    .arg("-C")
                    .arg(&root)
                    .args(arguments)
                    .status()
                    .expect("git executable")
                    .success()
            );
        };
        git(&["init", "-q"]);
        git(&["add", "-A"]);
        git(&[
            "-c",
            "user.name=Scriberig Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            "fixture",
        ]);
        let store = Arc::new(SqliteStore::open(&database).expect("database"));
        let backend = ApplicationBackendComposition::new(
            StoreInspectionBackend::new(Arc::clone(&store)),
            StoreOperationProvider::new(Arc::clone(&store)),
        );
        let service = ApplicationService::new(backend);
        let mut actions = [
            "runtime_capabilities",
            "inspect",
            "project_scan",
            "project_manifest",
            "project_import",
            "project_workspace",
            "semantic_find",
            "semantic_render",
            "agent_start",
        ]
        .into_iter()
        .map(|name| CapabilityAction::Custom(name.to_owned()))
        .collect::<std::collections::BTreeSet<_>>();
        actions.insert(CapabilityAction::Inspect);
        actions.insert(CapabilityAction::Query);
        let context = ApplicationContext {
            capabilities: CapabilityContext {
                granted: vec![
                    scriberig_app::CapabilityId::new("inspect.entity").expect("inspect capability"),
                ],
                grants: vec![Capability {
                    scope: CapabilityScope::Project,
                    actions,
                    constraints: CapabilityConstraints::unrestricted(),
                }],
            },
            ..ApplicationContext::empty().with_bootstrap(BootstrapEnvironment {
                working_directory: root.clone(),
            })
        };
        let session = EntityId::new();
        let handle = service
            .begin_agent_execution(
                &context,
                &AgentCommandRequest {
                    session,
                    branch: None,
                    prompt: "Onboard the current project.".to_owned(),
                    selection: None,
                },
            )
            .expect("authorized project-less operation");
        let result = execute_agent(
            BootstrapProvider,
            "deterministic-bootstrap",
            &service,
            &store,
            &context,
            session,
            handle.root_branch,
            "Onboard the current project.",
            None,
        )
        .expect("normal agent orchestration");
        assert!(result.output.contains("semantic root was inspected"));
        let tools = result
            .messages
            .iter()
            .filter_map(|message| match message {
                AgentMessage::Tool(result) => Some(result),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(tools.len(), 7);
        assert!(tools[1].summary.as_ref().unwrap().contains("110 files"));
        let project: ProjectId = tools[1].handles[0].parse().expect("project identity");
        let manifest_id = tools[1].handles[1].clone();
        assert!(tools[2].summary.as_ref().unwrap().contains(&manifest_id));
        assert!(
            tools[2]
                .handles
                .iter()
                .any(|entry| entry == "next_offset:100")
        );
        assert!(
            tools[3]
                .summary
                .as_ref()
                .unwrap()
                .contains("page starts at 100")
        );
        assert!(
            tools[2]
                .handles
                .iter()
                .any(|entry| entry.contains("Cargo.toml"))
        );
        assert!(tools[4].summary.as_ref().unwrap().contains("snapshot"));
        assert_eq!(tools[5].handles[0], project.to_string());
        let snapshot: SnapshotId = tools[5].handles[1].parse().expect("snapshot identity");
        let root_entity: EntityId = tools[5].handles[2].parse().expect("root identity");
        assert_eq!(tools[6].entity, root_entity);
        assert_eq!(tools[6].snapshot, Some(snapshot));
        let semantic_context = context.clone().with_snapshot(snapshot);
        let semantic_tools = LocalAgentTool::new(&service).with_project_store(Arc::clone(&store));
        let find = semantic_tools
            .invoke_named(
                &semantic_context,
                "semantic_find",
                &serde_json::json!({
                    "project": project.to_string(),
                    "snapshot": snapshot.to_string(),
                    "query": "AGENTS.md",
                    "limit": 10
                }),
            )
            .expect("find guidance semantically");
        let guidance = find
            .handles
            .iter()
            .find(|value| value.contains("AGENTS.md"))
            .expect("semantic guidance handle");
        let entity: EntityId = serde_json::from_str::<serde_json::Value>(guidance)
            .expect("semantic handle")
            .get("entity")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("semantic entity missing from {guidance}"))
            .parse()
            .expect("entity identity");
        let rendered = semantic_tools
            .invoke_named(
                &semantic_context,
                "semantic_render",
                &serde_json::json!({
                    "project": project.to_string(),
                    "snapshot": snapshot.to_string(),
                    "entity": entity.to_string()
                }),
            )
            .expect("render guidance semantically");
        assert!(
            rendered
                .summary
                .as_ref()
                .is_some_and(|text| text.contains("Use semantic project references"))
        );
        assert_semantic_architecture(&semantic_tools, &semantic_context, project, snapshot);
        let project_record = store
            .project(project)
            .expect("project record")
            .expect("onboarded project");
        assert_eq!(project_record.current_snapshot, Some(snapshot));
        assert!(
            store
                .project_entities(project)
                .expect("owned entities")
                .iter()
                .any(|entity| entity.entity == root_entity)
        );
        service
            .complete_local_agent(&handle, result.output)
            .expect("durable result");
        drop(service);
        drop(store);
        fs::remove_file(database).expect("remove database");
        fs::remove_dir_all(root).expect("remove fixture");
    }

    /// Copies real architecture entry points into the semantic bootstrap fixture.
    ///
    /// # Parameters
    /// - `root`: Project fixture that receives repository-relative source copies.
    /// - `super`: Parent test module containing the bootstrap scenario.
    ///
    /// # Panics
    /// Panics when a required Scriberig source or fixture directory cannot be copied.
    fn add_architecture_fixture(root: &std::path::Path) {
        let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for relative in [
            "Cargo.toml",
            "capabilities.toml",
            "crates/domains/scriberig-domain-code/src/lib.rs",
            "crates/domains/scriberig-domain-python/src/lib.rs",
            "crates/domains/test-support/src/conformance.rs",
            "crates/scriberig-project/src/adapters.rs",
            "crates/scriberig-project/src/scan.rs",
        ] {
            let source = repository.join(relative);
            let target = root.join(relative);
            fs::create_dir_all(target.parent().expect("architecture parent"))
                .expect("create architecture fixture directories");
            fs::copy(source, target).expect("copy real architecture source");
        }
    }

    /// Discovers architecture files by semantic source path and exact snapshot.
    ///
    /// # Parameters
    /// - `tools`: Shared project semantic application tool adapter.
    /// - `context`: Trusted exact-snapshot context for the imported project.
    /// - `project`: Stable imported project identity.
    /// - `snapshot`: Exact immutable project snapshot.
    /// - `super`: Parent test module containing the bootstrap scenario.
    ///
    /// # Panics
    /// Panics when a representative architecture source cannot be found or rendered.
    fn assert_semantic_architecture(
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
                .unwrap_or_else(|| {
                    panic!("exact semantic architecture handle for {path}: {found:?}")
                });
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

    /// Rejects unauthorized scans and model-supplied filesystem paths.
    #[test]
    fn bootstrap_scan_requires_capability_and_trusted_path_reference() {
        let root = std::env::temp_dir().join(format!("scriberig-agent-path-{0}", EntityId::new()));
        fs::create_dir_all(&root).expect("fixture root");
        let database = root.with_extension("sqlite");
        let store = Arc::new(SqliteStore::open(&database).expect("database"));
        let backend = ApplicationBackendComposition::new(
            StoreInspectionBackend::new(Arc::clone(&store)),
            StoreOperationProvider::new(Arc::clone(&store)),
        );
        let service = ApplicationService::new(backend);
        let tools = LocalAgentTool::new(&service).with_project_store(Arc::clone(&store));
        let bootstrap = BootstrapEnvironment {
            working_directory: root.clone(),
        };
        let unauthorized = ApplicationContext::empty().with_bootstrap(bootstrap.clone());
        assert!(matches!(
            tools.invoke_named(
                &unauthorized,
                "project_scan",
                &serde_json::json!({"path_ref": "current_working_directory"})
            ),
            Err(scriberig_agent_core::ToolError::CapabilityDenied { .. })
        ));
        let authorized = ApplicationContext {
            capabilities: CapabilityContext::with_grants([Capability {
                scope: CapabilityScope::Project,
                actions: [CapabilityAction::Custom("project_scan".to_owned())]
                    .into_iter()
                    .collect(),
                constraints: CapabilityConstraints::unrestricted(),
            }]),
            ..ApplicationContext::empty().with_bootstrap(bootstrap)
        };
        assert!(matches!(
            tools.invoke_named(
                &authorized,
                "project_scan",
                &serde_json::json!({"path_ref": "/tmp/untrusted"})
            ),
            Err(scriberig_agent_core::ToolError::Application { category, .. })
                if category == "invalid_request"
        ));
        assert_eq!(store.list_projects().expect("list projects").len(), 0);
        drop(tools);
        drop(service);
        drop(store);
        fs::remove_file(database).expect("remove database");
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
