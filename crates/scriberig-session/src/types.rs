//! Defines session-level entities and cross-domain handles.
use crate::branch::Branch;
use scriberig_core::{EntityId, Provenance};
#[derive(Clone, Debug, Eq, PartialEq)]
/// Session-domain structured value.
pub struct Session {
    pub id: EntityId,
    pub branches: Vec<Branch>,
    pub goals: Vec<Goal>,
    pub tasks: Vec<Task>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
/// Session-domain structured value.
pub struct Goal {
    pub id: EntityId,
    pub owner: EntityId,
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
/// Session-domain structured value.
pub struct Task {
    pub id: EntityId,
    pub owner: EntityId,
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
/// Session-domain structured value.
pub struct Observation {
    pub id: EntityId,
    pub text: String,
    pub provenance: Provenance,
}
#[derive(Clone, Debug, Eq, PartialEq)]
/// Session-domain structured value.
pub struct Question {
    pub id: EntityId,
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
/// Session-domain structured value.
pub struct Decision {
    pub id: EntityId,
    pub branch: EntityId,
    pub text: String,
    pub provenance: Provenance,
}
#[derive(Clone, Debug, Eq, PartialEq)]
/// Session-domain structured value.
pub struct BranchResult {
    pub id: EntityId,
    pub branch: EntityId,
    pub references: Vec<EntityId>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
/// Session-domain structured value.
pub struct Summary {
    pub id: EntityId,
    pub text: String,
    pub provenance: Provenance,
}
