//! Verifies bounded child runtime tools and capability delegation.

use super::{compile_child_context, inspect_catalog};
use scriberig_agent_runtime::context::ProjectSemanticScope;
use scriberig_agent_runtime::{ChildExecutionScope, ChildRequest};
use scriberig_app::{
    Capability, CapabilityAction, CapabilityConstraints, CapabilityContext, CapabilityId,
    CapabilityScope,
};
use scriberig_core::EntityId;

#[path = "child_runtime_tests/nested.rs"]
mod nested;

/// Verifies children without project scope still receive profile guidance.
#[test]
fn child_without_project_scope_gets_core_and_handoff_context() {
    let child_branch = EntityId::from_bytes([71; 16]);
    let parent_branch = EntityId::from_bytes([72; 16]);
    let scope = ChildExecutionScope {
        session: EntityId::from_bytes([73; 16]),
        parent_branch,
        depth: 1,
        active_branches: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        semantic_scope: None,
    };
    let request = ChildRequest {
        purpose: "inspect child path".to_owned(),
        prompt: "inspect the code".to_owned(),
        capabilities: Vec::new(),
        expected_result: "compact findings".to_owned(),
    };

    let compiled = compile_child_context(&scope, child_branch, &request);

    assert!(
        compiled
            .canonical
            .contains("scriberig.context-component.core-invariants@")
    );
    assert!(
        compiled
            .canonical
            .contains("scriberig.context-component.non-root-handoff@")
    );
    assert!(!compiled.canonical.contains("semantic_project_scope"));
}

/// Verifies a scoped child keeps its exact project selection and handoff.
#[test]
fn child_with_project_scope_keeps_exact_scope_and_handoff_context() {
    let scope = ChildExecutionScope {
        session: EntityId::from_bytes([81; 16]),
        parent_branch: EntityId::from_bytes([82; 16]),
        depth: 2,
        active_branches: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        semantic_scope: Some(ProjectSemanticScope {
            project: "project-exact".to_owned(),
            snapshot: "snapshot-exact".to_owned(),
            root: "root-exact".to_owned(),
            selection: Some("entity-exact".to_owned()),
        }),
    };
    let request = ChildRequest {
        purpose: "inspect scoped child path".to_owned(),
        prompt: "inspect selected code".to_owned(),
        capabilities: Vec::new(),
        expected_result: "compact findings".to_owned(),
    };

    let compiled = compile_child_context(&scope, EntityId::from_bytes([83; 16]), &request);

    assert!(compiled.canonical.contains("project=project-exact"));
    assert!(compiled.canonical.contains("snapshot=snapshot-exact"));
    assert!(compiled.canonical.contains("root=root-exact"));
    assert!(compiled.canonical.contains("selection=entity-exact"));
    assert!(
        compiled
            .canonical
            .contains("scriberig.context-component.core-invariants@")
    );
    assert!(
        compiled
            .canonical
            .contains("scriberig.context-component.non-root-handoff@")
    );
}

/// Verifies project recovery tools are exposed to provider runtimes.
#[test]
fn catalog_exposes_project_recovery_operations() {
    let names = inspect_catalog()
        .definitions()
        .into_iter()
        .map(|definition| definition.name)
        .collect::<Vec<_>>();
    for name in [
        "project_import",
        "project_manifest",
        "project_repair_file",
        "project_rescan",
        "project_scan",
        "project_workspace",
    ] {
        assert!(names.iter().any(|candidate| candidate == name));
    }
    let import = inspect_catalog()
        .definitions()
        .into_iter()
        .find(|definition| definition.name == "project_import")
        .expect("project import descriptor");
    assert_eq!(
        import.parameters["required"],
        serde_json::json!(["project", "manifest_id"])
    );
}

/// Keeps unrestricted shell and raw-file writes out of the worker catalog.
#[test]
fn worker_catalog_does_not_grant_unrestricted_shell_or_file_writes() {
    let names = inspect_catalog()
        .definitions()
        .into_iter()
        .map(|definition| definition.name)
        .collect::<Vec<_>>();
    assert!(
        !names
            .iter()
            .any(|name| { matches!(name.as_str(), "shell" | "execute_shell" | "raw_file_write") })
    );
}

/// Exposes bounded dependency acquisition as an independently granted operation.
#[test]
fn catalog_exposes_locked_dependency_acquisition() {
    let descriptor = inspect_catalog()
        .descriptors_for(&CapabilityContext::new([CapabilityId::new(
            "project_resolve_dependencies",
        )
        .expect("dependency capability")]))
        .into_iter()
        .find(|descriptor| descriptor.definition.name == "project_resolve_dependencies")
        .expect("dependency acquisition descriptor");
    assert!(descriptor.state_changing);
    assert_eq!(
        descriptor.definition.parameters["required"],
        serde_json::json!(["candidate"])
    );
}

/// Advertises semantic mutation through a trusted provider-neutral category.
#[test]
fn catalog_exposes_installed_semantic_operation_entrypoint() {
    let descriptor = inspect_catalog()
        .descriptors_for(&CapabilityContext::new([CapabilityId::new(
            "domain_operation",
        )
        .expect("domain operation capability")]))
        .into_iter()
        .find(|descriptor| descriptor.definition.name == "domain_operation")
        .expect("semantic operation descriptor");

    assert!(descriptor.state_changing);
    assert_eq!(
        descriptor.category,
        scriberig_agent_runtime::tools::ToolCategory::SemanticMutation
    );
    assert_eq!(
        descriptor.definition.parameters["required"],
        serde_json::json!(["capability", "entity", "snapshot"])
    );
    assert_eq!(
        descriptor.definition.parameters["properties"]["arguments"]["additionalProperties"]["x-sensitive"],
        true
    );
    let ordered = inspect_catalog()
        .descriptors_for(&CapabilityContext::new([
            CapabilityId::new("domain_operation").expect("domain capability"),
            CapabilityId::new("project_repair_file").expect("fallback capability"),
        ]))
        .into_iter()
        .map(|descriptor| (descriptor.definition.name, descriptor.category))
        .collect::<Vec<_>>();
    let semantic_position = ordered
        .iter()
        .position(|(name, _)| name == "domain_operation")
        .expect("semantic operation position");
    let fallback_position = ordered
        .iter()
        .position(|(name, _)| name == "project_repair_file")
        .expect("fallback position");
    assert!(semantic_position < fallback_position);
}

/// Restricts the bootstrap catalog to capabilities granted to one branch.
#[test]
fn catalog_filters_bootstrap_tools_by_branch_capability() {
    let capabilities = CapabilityContext::new([
        CapabilityId::new("project_scan").expect("scan capability"),
        CapabilityId::new("inspect.entity").expect("inspect capability"),
    ]);
    let names = inspect_catalog()
        .descriptors_for(&capabilities)
        .into_iter()
        .map(|descriptor| descriptor.definition.name)
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["inspect", "project_scan"]);
}

/// Keeps child capabilities within the explicit read-only delegation.
#[test]
fn child_capabilities_do_not_inherit_parent_mutations() {
    let parent = CapabilityContext {
        granted: vec![
            CapabilityId::new("inspect.entity").expect("inspect capability"),
            CapabilityId::new("project_import").expect("import capability"),
        ],
        grants: vec![Capability {
            scope: CapabilityScope::Project,
            actions: [
                CapabilityAction::Inspect,
                CapabilityAction::Custom("project_import".to_owned()),
                CapabilityAction::Custom("semantic_find".to_owned()),
                CapabilityAction::Custom("semantic_render".to_owned()),
            ]
            .into_iter()
            .collect(),
            constraints: CapabilityConstraints::unrestricted(),
        }],
    };
    let child = super::delegated_capabilities(
        &parent,
        &[
            "inspect.entity".to_owned(),
            "semantic_find".to_owned(),
            "semantic_render".to_owned(),
        ],
    );
    let names = inspect_catalog()
        .descriptors_for(&child)
        .into_iter()
        .map(|descriptor| descriptor.definition.name)
        .collect::<Vec<_>>();

    assert!(names.contains(&"inspect".to_owned()));
    assert!(names.contains(&"semantic_find".to_owned()));
    assert!(names.contains(&"semantic_render".to_owned()));
    assert!(!names.contains(&"project_import".to_owned()));
    assert!(!names.contains(&"project_repair_file".to_owned()));
    assert!(!names.contains(&"project_rescan".to_owned()));
    assert!(!child.allows(&CapabilityId::new("project_import").expect("id")));
}

/// Keeps candidate construction and validation available without delegating Apply.
#[test]
fn implementation_child_can_validate_but_cannot_apply_candidates() {
    let parent = CapabilityContext {
        granted: [
            "project_propose",
            "project_validate_candidate",
            "project_apply",
        ]
        .into_iter()
        .map(|id| CapabilityId::new(id).expect("capability identity"))
        .collect(),
        grants: vec![Capability {
            scope: CapabilityScope::Project,
            actions: [
                "project_propose",
                "project_validate_candidate",
                "project_apply",
            ]
            .into_iter()
            .map(|name| CapabilityAction::Custom(name.to_owned()))
            .collect(),
            constraints: CapabilityConstraints::unrestricted(),
        }],
    };
    let child = super::delegated_capabilities(
        &parent,
        &[
            "project_propose".to_owned(),
            "project_validate_candidate".to_owned(),
        ],
    );
    let names = inspect_catalog()
        .descriptors_for(&child)
        .into_iter()
        .map(|descriptor| descriptor.definition.name)
        .collect::<Vec<_>>();

    assert!(child.allows(&CapabilityId::new("project_propose").expect("propose id")));
    assert!(child.allows(&CapabilityId::new("project_validate_candidate").expect("validation id")));
    assert!(!child.allows(&CapabilityId::new("project_apply").expect("apply id")));
    assert!(names.contains(&"project_propose".to_owned()));
    assert!(names.contains(&"project_validate_candidate".to_owned()));
    assert!(!names.contains(&"project_apply".to_owned()));
}
