//! Splits semantic profile application operations by model, storage, and orchestration.

mod model;
mod operations;
mod provider;
mod storage;

const PROFILE_TYPE: &str = "agent.profile";
const COMPONENT_TYPE: &str = "agent.context_component";
const PROFILE_PROPOSE: &str = "agent_profile.propose";
const PROFILE_PROMOTE: &str = "agent_profile.promote";

pub use model::{AgentProfileError, AgentProfileStoreResult, ProfileCandidate, ProfileInspection};
pub use operations::AgentProfileService;
pub use provider::AgentProfileProvider;
