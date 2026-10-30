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
#[path = "state_reducer/artifacts.rs"]
mod artifacts;
#[path = "state_reducer/cad.rs"]
mod cad;
#[path = "state_reducer/document.rs"]
mod document;
#[path = "state_reducer/document_edit.rs"]
mod document_edit;
#[path = "state_reducer/geo.rs"]
mod geo;
#[path = "state_reducer/geo_query.rs"]
mod geo_query;
#[path = "state_reducer/project_source_request.rs"]
mod project_source_request;
#[path = "state_reducer/projection_style.rs"]
mod projection_style;
#[path = "state_reducer/qgis.rs"]
mod qgis;
#[path = "state_reducer/qgis_order.rs"]
mod qgis_order;
#[path = "state_reducer/run.rs"]
mod run;
#[path = "state_reducer/spreadsheet.rs"]
mod spreadsheet;
#[path = "state_reducer/spreadsheet_cell.rs"]
mod spreadsheet_cell;
#[path = "state_reducer/spreadsheet_edit.rs"]
mod spreadsheet_edit;
#[path = "state_reducer_failure.rs"]
mod state_reducer_failure;
#[path = "state_reducer/tail_actions.rs"]
mod tail_actions;
#[path = "state_reducer/view.rs"]
mod view;
pub use run::reduce;
