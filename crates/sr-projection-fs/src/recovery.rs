//! Defines retryable failed-import evidence.
use crate::{FileDigest, ProjectionLocation, ProjectionRef};
use sr_projection::SemanticStateRef;
/// Stage at which an external import stopped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureStage {
    Read,
    Parse,
    Reconcile,
    Validate,
}
/// Immutable information required to retry a failed import.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryRecord {
    pub projection: ProjectionRef,
    pub base: SemanticStateRef,
    pub current: SemanticStateRef,
    pub external: FileDigest,
    pub location: ProjectionLocation,
    pub stage: FailureStage,
    pub diagnostics: String,
    pub bytes: Option<Vec<u8>>,
}
