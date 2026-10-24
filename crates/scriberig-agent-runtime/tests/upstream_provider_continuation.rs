//! Proves the host-owned provider continuation boundary for one branch run.
//!
//! `ScribeRig` owns whether a handle is offered to the upstream loop; upstream
//! owns the opaque handle, its extraction, and its consumption. These tests pin
//! that split with a deterministic provider that advertises continuation, so an
//! out-of-scope or unsupported handle fails closed before any provider request.

#[path = "upstream_provider_continuation/fixture.rs"]
mod fixture;
#[path = "upstream_provider_continuation/lifecycle.rs"]
mod lifecycle;
