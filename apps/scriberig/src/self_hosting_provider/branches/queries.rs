//! Stores stable semantic query targets used by self hosting scenarios.

/// Read-only source references inspected by the architecture branch.
pub(super) const ARCHITECTURE_QUERIES: &[&str] = &[
    "crates/domains/scriberig-domain-python/src/lib.rs",
    "design/level-2/028",
    "crates/scriberig-project/src/scan.rs",
    "crates/scriberig-project/src/support.rs",
    "crates/scriberig-project/src/adapters.rs",
    "Cargo.toml",
    "crates/scriberig-project/tests",
];

/// Exact target sources re-inspected by the implementation branch.
pub(super) const IMPLEMENTATION_QUERIES: &[&str] = &[
    "Cargo.toml",
    "crates/scriberig-project/Cargo.toml",
    "crates/scriberig-project/src/scan.rs",
    "crates/scriberig-project/src/adapters.rs",
    "crates/scriberig-project/src/support.rs",
];

/// Registered validation profiles required for the PHP self-hosting candidate.
pub(super) const PHP_CANDIDATE_PROFILES: &[&str] = &[
    "php-domain-focused",
    "project-import-focused",
    "rust-workspace",
    "project-ingestion-e2e",
];
