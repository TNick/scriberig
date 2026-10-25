//! Proves the host-owned provider continuation boundary for one branch run.
//!
//! `ScribeRig` owns whether a handle is offered to the upstream loop; upstream
//! owns the opaque handle, its extraction, and its consumption. These tests pin
//! that split with a deterministic provider that advertises continuation, so an
//! out-of-scope or unsupported handle fails closed before any provider request.
//! `owner` covers the `ScribeRig` handle owner itself: fresh seed, same-lineage
//! reuse through a real provider request, binding divergence, and failure or
//! cancellation clearing. `owner_scope` covers the same owner's isolation,
//! scope-rejection, restart seed, and non-persistence rules. `reseed` drives the
//! same decisions with the compiled-context fingerprint and provider scope the
//! invocation seam itself computes, and proves each reseed as a cold canonical
//! dispatch. `execution_scope` pins that delegated child branches and durable
//! plan workers never receive, consume, or replace the root lineage's handle.
//! `reseed_lineage` and `reseed_provider` are the split of the seam-computed
//! reseed cases so each file stays inside the repository file-size rule.

#[path = "upstream_provider_continuation/execution_scope.rs"]
mod execution_scope;
#[path = "upstream_provider_continuation/fixture.rs"]
mod fixture;
#[path = "upstream_provider_continuation/lifecycle.rs"]
mod lifecycle;
#[path = "upstream_provider_continuation/owner.rs"]
mod owner;
#[path = "upstream_provider_continuation/owner_scope.rs"]
mod owner_scope;
#[path = "upstream_provider_continuation/reseed_lineage.rs"]
mod reseed_lineage;
#[path = "upstream_provider_continuation/reseed_provider.rs"]
mod reseed_provider;
