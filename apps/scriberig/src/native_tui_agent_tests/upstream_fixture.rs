//! Composes the native runner with one scripted upstream provider fixture.
//!
//! The fixture keeps the production root-execution path and only replaces the
//! upstream provider, so durable activity ordering, correlation, and
//! cancellation settle through the same `ScribeRig` boundaries as production.

#[path = "upstream_fixture/fixture.rs"]
pub(crate) mod fixture;
#[path = "upstream_fixture/provider.rs"]
pub(crate) mod provider;
