//! Defines stable session vocabulary names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Session-domain structured value.
pub enum SessionEntityType {
    Session,
    Branch,
    Goal,
    HumanInput,
    AgentOutput,
    Observation,
    Question,
    Decision,
    ToolInvocation,
    ToolResult,
    BranchResult,
    Summary,
    Task,
    Turn,
}
impl SessionEntityType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Session => "Session",
            Self::Branch => "Branch",
            Self::Goal => "Goal",
            Self::HumanInput => "HumanInput",
            Self::AgentOutput => "AgentOutput",
            Self::Observation => "Observation",
            Self::Question => "Question",
            Self::Decision => "Decision",
            Self::ToolInvocation => "ToolInvocation",
            Self::ToolResult => "ToolResult",
            Self::BranchResult => "BranchResult",
            Self::Summary => "Summary",
            Self::Task => "Task",
            Self::Turn => "Turn",
        }
    }
}
