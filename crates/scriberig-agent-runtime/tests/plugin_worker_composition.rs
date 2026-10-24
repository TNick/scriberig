//! Proves the `ScribeRig` composition seam between an upstream isolated plugin
//! worker and one branch tool set.
//!
//! The pinned upstream API supplies the worker process, the versioned protocol,
//! the host-call allowlist, and the injected sandbox launcher contract. These
//! tests pin the `ScribeRig` obligations on top of that seam: plugin tools are
//! admitted only through an explicitly capability-scoped catalog registration,
//! every call is reauthorized before it reaches the worker, no protected path or
//! ambient host credential enters the worker process, and branch cancellation
//! reaches the worker as a cancel frame. They run only when the opt-in
//! `isolated-plugin-worker` feature is enabled.
//!
//! What these tests do not claim: the launcher guarantee set is a launcher
//! self-report, and the fixture launcher enforces no OS sandbox. Product
//! activation still needs a verified platform sandbox launcher with runtime
//! evidence, so `ScribeRig` keeps native plugin activation rejected in every
//! default build.

#![cfg(feature = "isolated-plugin-worker")]

#[path = "plugin_worker_composition/cancellation.rs"]
mod cancellation;
#[path = "plugin_worker_composition/catalog.rs"]
mod catalog;
#[path = "plugin_worker_composition/containment.rs"]
mod containment;
#[path = "plugin_worker_composition/denial.rs"]
mod denial;
#[path = "plugin_worker_composition/fixture.rs"]
mod fixture;
#[path = "plugin_worker_composition/helpers.rs"]
mod helpers;
#[path = "plugin_worker_composition/settlement.rs"]
mod settlement;
