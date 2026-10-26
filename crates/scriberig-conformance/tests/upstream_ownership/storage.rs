//! Manifest rules that keep durable storage behind the shared operation owners.
//!
//! The task-150 plan write boundary has two halves. Only the SQLite backend
//! crates may link the SQLite driver, so no other member can open its own SQL
//! write path. The shared-operation surfaces - the remote client and the terminal -
//! may not declare a storage backend at all, so their plan reads and mutations
//! must travel through `scriberig-app` operations and the `scriberig-session`
//! store contract.

use crate::manifest::{
    Declaration, member_declarations, read_toml, workspace_dependency_table, workspace_members,
    workspace_root,
};

/// Workspace members allowed to link the SQLite driver directly.
///
/// Task 150 keeps one canonical plan write path: durable plan state is promoted
/// only through the semantic-root compare-and-swap inside these backends, so a
/// member that links `rusqlite` itself owns a direct SQL write path and defeats
/// that boundary. Other members may consume a backend crate, but only these three
/// can hold the driver.
const SQLITE_BACKEND_OWNERS: [&str; 3] = [
    "crates/scriberig-session-sqlite",
    "crates/scriberig-store-sqlite",
    "crates/scriberig-style-sqlite",
];

/// Members that must consume semantics through shared operations instead of a
/// storage backend.
///
/// The terminal and remote-client surfaces present the shared plan, session, and
/// review views and emit shared `scriberig-app` requests; neither has storage
/// authority, so a plan read or mutation from them cannot bypass the shared
/// operation boundary.
const SHARED_OPERATION_SURFACE_MEMBERS: [&str; 2] =
    ["crates/scriberig-client", "crates/scriberig-tui"];

/// Collects the SQLite driver requests resolved from one manifest.
///
/// # Parameters
/// - `owner`: manifest the declarations belong to, used in the message.
/// - `declarations`: resolved dependency declarations to inspect.
///
/// # Returns
/// One message per direct SQLite driver request.
fn driver_declarations(owner: &str, declarations: &[Declaration]) -> Vec<String> {
    let mut failures: Vec<String> = Vec::new();
    for declaration in declarations {
        for name in &declaration.names {
            if name == "rusqlite" {
                failures.push(declaration.violation(owner, name));
            }
        }
    }
    failures
}

/// Collects the storage-backend crate requests resolved from one manifest.
///
/// # Parameters
/// - `owner`: manifest the declarations belong to, used in the message.
/// - `declarations`: resolved dependency declarations to inspect.
///
/// # Returns
/// One message per SQLite backend crate request.
fn backend_declarations(owner: &str, declarations: &[Declaration]) -> Vec<String> {
    let mut failures: Vec<String> = Vec::new();
    for declaration in declarations {
        for name in &declaration.names {
            if name.ends_with("-sqlite") {
                failures.push(declaration.violation(owner, name));
            }
        }
    }
    failures
}

/// Verifies only the SQLite storage backends link the SQLite driver.
#[test]
fn only_storage_backends_link_the_sqlite_driver() {
    let root = workspace_root();
    let workspace_deps = workspace_dependency_table(&root);
    let mut failures: Vec<String> = Vec::new();
    for member in workspace_members(&root) {
        if SQLITE_BACKEND_OWNERS.contains(&member.as_str()) {
            continue;
        }
        let manifest = read_toml(&root.join(&member).join("Cargo.toml"));
        failures.extend(driver_declarations(
            &member,
            &member_declarations(&manifest, &workspace_deps),
        ));
    }
    assert!(
        failures.is_empty(),
        "members outside the SQLite backends link the driver: {failures:?}"
    );
}

/// Verifies the terminal and remote-client surfaces declare no storage backend.
///
/// A plan action from either surface must travel through the shared application
/// operations, so a storage request here would create a second, direct write path.
#[test]
fn shared_operation_surfaces_declare_no_storage_backend() {
    let root = workspace_root();
    let workspace_deps = workspace_dependency_table(&root);
    let mut failures: Vec<String> = Vec::new();
    for member in SHARED_OPERATION_SURFACE_MEMBERS {
        let manifest = read_toml(&root.join(member).join("Cargo.toml"));
        failures.extend(backend_declarations(
            member,
            &member_declarations(&manifest, &workspace_deps),
        ));
    }
    assert!(
        failures.is_empty(),
        "shared-operation surfaces own storage: {failures:?}"
    );
}
