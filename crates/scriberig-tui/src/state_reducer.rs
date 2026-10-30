//! Reduces TUI actions into state and effects.

#![allow(clippy::wildcard_imports)]
use super::state_actions::activate_current;
use super::state_cursor_reducer::reduce_cursor;
use super::state_reducer_tail::reduce_tail;
use super::*;
use crate::{overlay::palette::PaletteState, state::ConnectionState};

#[path = "state_input_reducer.rs"]
mod state_input_reducer;
use state_input_reducer::reduce_input;
#[path = "state_reducer_commit.rs"]
mod state_reducer_commit;
use state_reducer_commit::commit_succeeded;
#[path = "state_reducer/run.rs"]
mod run;
#[path = "state_reducer_failure.rs"]
mod state_reducer_failure;
#[path = "state_reducer/tail_actions.rs"]
mod tail_actions;
pub use run::reduce;
