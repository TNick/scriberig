//! Verifies durable request and tool-result detail reads.
//!
//! The detail operations must return only durable, bounded, authorized state.
//! An upstream applied-settings report keeps its omissions and completeness, and
//! catalog-derived execution metadata is never presented as an applied report.

#[path = "session_detail/applied_settings.rs"]
mod applied_settings;
#[path = "session_detail/detail.rs"]
mod detail;
#[path = "session_detail/fixture.rs"]
mod fixture;
