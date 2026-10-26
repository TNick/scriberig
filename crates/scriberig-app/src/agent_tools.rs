//! Provides application-backed provider-neutral agent tools.

mod inspect;

pub use crate::capability::{INSPECT_CAPABILITY, inspect_available};
pub use inspect::InspectTool;
