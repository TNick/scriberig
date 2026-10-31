//! Owns the ephemeral `ScribeRig` lifecycle of upstream provider continuations.
//!
//! Upstream `rpi-ai` owns the opaque handle, its extraction, and its provider
//! wire consumption. The handle carries only provider/model scope, so it cannot
//! state whether it belongs to the conversation that is about to be resumed;
//! `ScribeRig` owns that decision. This module is the single in-process owner of
//! that decision: it binds each handle to one exact execution identity, hands it
//! back only while that identity is unchanged, and drops it on divergence,
//! failed completion, or cancellation.
//!
//! Handles are process-local and never become `ScribeRig` identity. Nothing here
//! is serializable, no handle reaches a durable session, branch, plan, request
//! projection, or activity record, and a lost handle is always recovered by
//! reseeding from canonical compiled `ScribeRig` context.
//!
//! The owner key is the caller's execution identity, currently one canonical
//! root branch. Reuse across a delegated child branch or a durable plan worker
//! additionally requires the exact child/worker validity value recorded in
//! `design/level-3/146. replace local provider tool loop and extension host with
//! upstream agent and plugin runtime.md`; branch identity alone does not prove
//! that the anchor, plan claim, dependency results, and compiled context still
//! match, so those executions stay stateless.

mod binding;
mod owner;

pub use binding::ContinuationBinding;
pub use owner::{CompletedContinuation, ContinuationOutcome, EphemeralContinuationOwner};
