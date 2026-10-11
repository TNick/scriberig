//! Provides a reusable Rust client boundary over the public Scriberig protocol.

pub mod client;
pub mod error;
pub mod operation;
pub mod subscription;

pub use client::Client;
pub use error::ClientError;
pub use operation::OperationPoller;
pub use subscription::EventSubscription;
