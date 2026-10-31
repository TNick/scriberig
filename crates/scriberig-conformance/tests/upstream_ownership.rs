//! Guards the task-144 deletion ledger and the task-149 upstream-ownership limits.
//!
//! Task 149 removed the superseded local commodity owners (vendored `rpi-tui`,
//! `scriberig-tui-kit`, the Ratatui shell, the local terminal event/keymap
//! modules, and the local provider/loop/plugin ABI). These tests keep the
//! static half of that acceptance enforced instead of re-audited by hand: they
//! read the workspace manifests, `Cargo.lock`, `THIRD_PARTY_NOTICES.md`, and the
//! source tree, and they fail when a removed owner reappears, when a `ScribeRig`
//! crate takes direct terminal or dynamic-loader ownership, or when the pinned
//! upstream revision stops being reproducible. They prove dependency
//! ownership only; the behavioral delegation fixtures in task 149 §§7-13 are
//! separate and stay open.
//!
//! `manifest` resolves which packages a manifest really requests, following
//! `workspace = true` inheritance and `package = "..."` renames; `dependencies`
//! holds the per-manifest ownership rules; `ledger` holds the `Cargo.lock` and
//! deleted-path rules; `package_lifecycle` holds the task-145 §9 package,
//! extension, and catalog-publication ownership rules; `pinning` holds the
//! upstream pin and feature-gate rules; and `storage` holds the task-150 rule
//! that keeps durable storage behind the shared operation owners. The `#[path]`
//! attributes keep this documented entry-point file and its focused modules
//! together, because rustc looks for the submodules of a `tests/<name>.rs` crate
//! root in `tests/` itself rather than in a directory of the same stem.

#[path = "upstream_ownership/dependencies.rs"]
mod dependencies;
#[path = "upstream_ownership/ledger.rs"]
mod ledger;
#[path = "upstream_ownership/manifest.rs"]
mod manifest;
#[path = "upstream_ownership/package_lifecycle.rs"]
mod package_lifecycle;
#[path = "upstream_ownership/pinning.rs"]
mod pinning;
#[path = "upstream_ownership/storage.rs"]
mod storage;
