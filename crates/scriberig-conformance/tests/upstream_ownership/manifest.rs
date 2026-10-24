//! Manifest reading and dependency-identity resolution for the ownership guard.
//!
//! A member manifest can request a crate it never names: `workspace = true`
//! inherits the entry recorded in the workspace `[workspace.dependencies]`
//! table, and `package = "..."` renames the crate behind a dependency key.
//! Reading only the keys a member writes therefore misses inherited or aliased
//! requests, so these helpers resolve every dependency table (normal, dev,
//! build, and target-specific) against the workspace table and hand the rules
//! the package identities a manifest actually declares. An inherited entry the
//! workspace table does not declare is still judged by the key the member
//! wrote, so resolution failure never widens what the guard accepts.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use toml::Value;

/// Workspace `[workspace.dependencies]` declarations keyed by dependency name.
pub(crate) type WorkspaceDependencies = BTreeMap<String, Value>;

/// Dependency tables Cargo reads in one manifest.
const DEPENDENCY_TABLES: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

/// One dependency declaration after `package` renaming and `workspace = true`
/// inheritance are resolved.
pub(crate) struct Declaration {
    /// Dependency key written in the manifest dependency table.
    pub(crate) key: String,
    /// Whether the entry requests its settings from `[workspace.dependencies]`.
    pub(crate) inherited: bool,
    /// Every package name the declaration can resolve to: the written key, the
    /// member-level `package` rename, and the inherited entry's rename.
    pub(crate) names: BTreeSet<String>,
}

impl Declaration {
    /// Resolves one dependency entry against the workspace dependency table.
    ///
    /// # Parameters
    /// - `key`: dependency key written in the manifest dependency table.
    /// - `entry`: declaration value written for that key.
    /// - `workspace_deps`: workspace `[workspace.dependencies]` table.
    ///
    /// # Returns
    /// The declaration together with every package name it resolves to.
    pub(crate) fn resolve(
        key: &str,
        entry: &Value,
        workspace_deps: &WorkspaceDependencies,
    ) -> Self {
        let mut names = BTreeSet::new();
        names.insert(key.to_owned());
        add_renamed_package(entry, &mut names);
        let inherited = entry.get("workspace").and_then(Value::as_bool) == Some(true);
        if inherited {
            if let Some(inherited_entry) = workspace_deps.get(key) {
                add_renamed_package(inherited_entry, &mut names);
            }
        }
        Self {
            key: key.to_owned(),
            inherited,
            names,
        }
    }

    /// Reports whether the declaration resolves to one package name.
    ///
    /// # Parameters
    /// - `wanted`: package name to look for.
    ///
    /// # Returns
    /// `true` when any resolved identity equals `wanted`.
    pub(crate) fn resolves_to(&self, wanted: &str) -> bool {
        self.names.iter().any(|name| name == wanted)
    }

    /// Collects the upstream `rpi-*` identities the declaration resolves to.
    ///
    /// # Returns
    /// The resolved package names belonging to the upstream `rpi-*` family.
    pub(crate) fn upstream_names(&self) -> Vec<String> {
        self.names
            .iter()
            .filter(|name| name.starts_with("rpi-"))
            .cloned()
            .collect()
    }

    /// Names one forbidden package identity in a failure message.
    ///
    /// # Parameters
    /// - `owner`: manifest the declaration belongs to.
    /// - `name`: forbidden package identity the declaration resolves to.
    ///
    /// # Returns
    /// A message naming the owner, whether the request was inherited, and the
    /// forbidden package.
    pub(crate) fn violation(&self, owner: &str, name: &str) -> String {
        let kind = if self.inherited {
            "inherits"
        } else {
            "declares"
        };
        format!("{owner} {kind} {name}")
    }
}

/// Adds a `package = "..."` rename to a set of package identities.
///
/// # Parameters
/// - `declaration`: dependency declaration that may rename its package.
/// - `names`: collection receiving the renamed package name.
fn add_renamed_package(declaration: &Value, names: &mut BTreeSet<String>) {
    if let Some(package) = declaration.get("package").and_then(Value::as_str) {
        names.insert(package.to_owned());
    }
}

/// Returns the workspace root directory for this crate.
///
/// # Returns
/// The directory holding `Cargo.toml`, `Cargo.lock`, and the member crates.
pub(crate) fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

/// Reads one TOML document from the workspace.
///
/// - `path`: Manifest or lockfile to parse.
///
/// # Returns
/// The parsed TOML document.
///
/// # Panics
/// Panics when the file is missing or is not valid TOML.
pub(crate) fn read_toml(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("cannot read {}: {error}", path.display());
    });
    text.parse().unwrap_or_else(|error| {
        panic!("cannot parse {}: {error}", path.display());
    })
}

/// Extracts the `[workspace.dependencies]` table of a parsed workspace manifest.
///
/// # Parameters
/// - `manifest`: parsed workspace manifest document.
///
/// # Returns
/// The declared workspace dependency entries keyed by dependency name.
///
/// # Panics
/// Panics when the manifest declares no `[workspace.dependencies]` table.
pub(crate) fn parse_workspace_dependencies(manifest: &Value) -> WorkspaceDependencies {
    manifest["workspace"]["dependencies"]
        .as_table()
        .expect("workspace dependencies")
        .clone()
        .into_iter()
        .collect()
}

/// Reads the workspace `[workspace.dependencies]` table.
///
/// # Parameters
/// - `root`: Workspace root holding the manifest.
///
/// # Returns
/// The declared workspace dependency entries keyed by dependency name.
pub(crate) fn workspace_dependency_table(root: &Path) -> WorkspaceDependencies {
    parse_workspace_dependencies(&read_toml(&root.join("Cargo.toml")))
}

/// Collects the member directories declared by the workspace manifest.
///
/// # Parameters
/// - `root`: Workspace root holding the manifest.
///
/// # Returns
/// Every workspace member path relative to the workspace root.
///
/// # Panics
/// Panics when the workspace manifest has no member list.
pub(crate) fn workspace_members(root: &Path) -> Vec<String> {
    let manifest = read_toml(&root.join("Cargo.toml"));
    manifest["workspace"]["members"]
        .as_array()
        .expect("workspace members")
        .iter()
        .map(|member| member.as_str().expect("member path").to_owned())
        .collect()
}

/// Resolves every dependency declaration in one member manifest.
///
/// # Parameters
/// - `manifest`: parsed member manifest document.
/// - `workspace_deps`: workspace `[workspace.dependencies]` table.
///
/// # Returns
/// One declaration per dependency key, across normal, dev, build, and
/// target-specific dependency tables.
pub(crate) fn member_declarations(
    manifest: &Value,
    workspace_deps: &WorkspaceDependencies,
) -> Vec<Declaration> {
    dependency_entries(manifest)
        .into_iter()
        .map(|(key, entry)| Declaration::resolve(&key, &entry, workspace_deps))
        .collect()
}

/// Resolves every entry in one `[workspace.dependencies]` table.
///
/// # Parameters
/// - `workspace_deps`: workspace dependency table to inspect.
///
/// # Returns
/// One declaration per workspace entry.
pub(crate) fn workspace_declarations(workspace_deps: &WorkspaceDependencies) -> Vec<Declaration> {
    let none = WorkspaceDependencies::new();
    workspace_deps
        .iter()
        .map(|(key, entry)| Declaration::resolve(key, entry, &none))
        .collect()
}

/// Collects the `(key, declaration)` pairs of every dependency table.
///
/// # Parameters
/// - `document`: parsed Cargo manifest document.
///
/// # Returns
/// Every dependency key with its declaration, including target-specific tables.
fn dependency_entries(document: &Value) -> Vec<(String, Value)> {
    let targets = document.get("target").and_then(Value::as_table);
    let containers: Vec<&Value> = std::iter::once(document)
        .chain(targets.into_iter().flat_map(|tables| tables.values()))
        .collect();
    let mut entries: Vec<(String, Value)> = Vec::new();
    for container in containers {
        for table_name in DEPENDENCY_TABLES {
            let Some(table) = container.get(table_name).and_then(Value::as_table) else {
                continue;
            };
            entries.extend(
                table
                    .iter()
                    .map(|(key, entry)| (key.clone(), entry.clone())),
            );
        }
    }
    entries
}

/// Returns the single git source revision pinned for the `rpi-*` crates.
///
/// # Parameters
/// - `root`: Workspace root holding the manifest.
///
/// # Returns
/// The pinned git URL and revision recorded in `[workspace.dependencies]`.
///
/// # Panics
/// Panics when no `rpi-*` git dependency is declared.
pub(crate) fn upstream_pin(root: &Path) -> (String, String) {
    let workspace_deps = workspace_dependency_table(root);
    for declaration in workspace_declarations(&workspace_deps) {
        if declaration.upstream_names().is_empty() {
            continue;
        }
        let entry = &workspace_deps[&declaration.key];
        let git = entry.get("git").and_then(Value::as_str).expect("git url");
        let rev = entry.get("rev").and_then(Value::as_str).expect("rev");
        return (git.to_owned(), rev.to_owned());
    }
    panic!("no rpi-* git dependency is pinned")
}
