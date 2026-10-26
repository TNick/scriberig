//! Proves the provider-boundary applied-settings provenance contract.
//!
//! Upstream owns the applied-settings report shape and its explicit omissions.
//! `ScribeRig` owns capturing that report on the durable model-request
//! projection and returning it only through the authorized detail operation.
//! Catalog metadata is separate provenance and is never reported as applied.

#[path = "applied_settings_provenance/capture.rs"]
mod capture;
#[path = "applied_settings_provenance/durable.rs"]
mod durable;
#[path = "applied_settings_provenance/fixture.rs"]
mod fixture;
