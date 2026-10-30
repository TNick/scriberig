//! Composes borrowed-service rich-artifact contract methods.

#[macro_use]
#[path = "server_borrowed_artifacts/reads.rs"]
mod reads;
#[macro_use]
#[path = "server_borrowed_artifacts/candidates.rs"]
mod candidates;

macro_rules! shared_borrowed_artifact_methods {
    () => {
        shared_borrowed_artifact_reads!();
        shared_borrowed_artifact_candidates!();
    };
}
