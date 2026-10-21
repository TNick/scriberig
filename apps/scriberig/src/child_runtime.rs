//! Exposes `ScribeRig` child-branch semantic policies and result projections.
#[path = "child_runtime/capability_filter.rs"]
mod capability_filter;
#[path = "child_runtime/control_parameters.rs"]
mod control_parameters;
pub(super) use capability_filter::delegated_capabilities;
#[path = "child_runtime/handles.rs"]
mod handles;
pub(super) use handles::compact_result_handles;

#[path = "child_runtime/catalog.rs"]
mod catalog;
pub(super) use catalog::inspect_catalog;

#[cfg(test)]
#[path = "child_runtime_tests.rs"]
mod tests;
