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
            "inspect",
            "project_scan",
            "project_manifest",
            "project_import",
            "project_workspace",
            "agent_start",
        ]
        .into_iter()
        .map(|name| CapabilityAction::Custom(name.to_owned()))
        .collect::<std::collections::BTreeSet<_>>();
        actions.insert(CapabilityAction::Inspect);
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
        assert_eq!(tools.len(), 5);
        assert!(tools[0].summary.as_ref().unwrap().contains("2 files"));
        let project: ProjectId = tools[0].handles[0].parse().expect("project identity");
        let manifest_id = tools[0].handles[1].clone();
        assert!(tools[1].summary.as_ref().unwrap().contains(&manifest_id));
        assert!(
            tools[1]
                .handles
                .iter()
                .any(|entry| entry.contains("Cargo.toml"))
        );
        assert!(tools[2].summary.as_ref().unwrap().contains("snapshot"));
        assert_eq!(tools[3].handles[0], project.to_string());
        let snapshot: SnapshotId = tools[3].handles[1].parse().expect("snapshot identity");
        let root_entity: EntityId = tools[3].handles[2].parse().expect("root identity");
        assert_eq!(tools[4].entity, root_entity);
        assert_eq!(tools[4].snapshot, Some(snapshot));
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
