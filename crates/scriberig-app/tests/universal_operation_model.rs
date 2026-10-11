//! Verifies universal application dispatch across read and operation providers.

#[path = "universal_operation_support.rs"]
mod support;
pub use support::*;

mod universal_operation_dispatch;
mod universal_read_operations;
