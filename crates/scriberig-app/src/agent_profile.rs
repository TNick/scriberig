//! Provides shared semantic read, candidate, and promotion operations for agent profiles.

mod service;
pub mod types;

pub use service::{
    AgentProfileError, AgentProfileProvider, AgentProfileService, AgentProfileStoreResult,
    ProfileCandidate, ProfileInspection,
};
