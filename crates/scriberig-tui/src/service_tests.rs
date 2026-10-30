//! Tests the module's deterministic behavior.

use super::{ApplicationAdapter, TuiService, execute_effect};
use crate::{action::Action, effect::Effect};
use scriberig_app::{
    ApplicationContext, ApplicationError, InspectRequest, InspectResult, QueryService,
    SessionBackend,
};
use scriberig_core::{EntityId, SnapshotId};

/// Provides a deterministic query result for adapter tests.
struct Fixture;

impl QueryService for Fixture {
    /// Returns a deterministic empty member page for the shared TUI adapter test.
    ///
    /// # Parameters
    /// - `self`: Query fixture.
    /// - `_context`: Application request context.
    /// - `project`: Stable owning project identity.
    /// - `artifact`: Stable semantic artifact identity.
    /// - `_offset`: Requested member offset.
    /// - `_limit`: Maximum number of members.
    ///
    /// # Returns
    /// Empty page with the requested owning identities.
    ///
    /// # Errors
    /// This fixture does not fail member inspection.
    fn inspect_artifact_members(
        &self,
        _context: &ApplicationContext,
        project: scriberig_core::ProjectId,
        artifact: EntityId,
        _offset: usize,
        _limit: usize,
    ) -> Result<scriberig_app::ArtifactMembers, ApplicationError> {
        Ok(scriberig_app::ArtifactMembers {
            project,
            artifact,
            members: Vec::new(),
            total: 0,
            next_offset: None,
        })
    }

    /// Returns a deterministic bounded artifact page for the TUI service test.
    ///
    /// # Parameters
    /// - `self`: Query fixture.
    /// - `_context`: Application request context.
    /// - `project`: Stable project identity.
    /// - `_offset`: Requested artifact offset.
    /// - `_limit`: Maximum artifact count.
    ///
    /// # Returns
    /// An empty artifact page for the requested project.
    ///
    /// # Errors
    /// This fixture does not fail artifact inspection.
    fn inspect_artifacts(
        &self,
        _context: &ApplicationContext,
        project: scriberig_core::ProjectId,
        _offset: usize,
        _limit: usize,
    ) -> Result<scriberig_app::ArtifactManifestPage, ApplicationError> {
        Ok(scriberig_app::ArtifactManifestPage {
            project,
            artifacts: Vec::new(),
            synchronization: Vec::new(),
            total: 0,
            next_offset: None,
        })
    }

    /// Returns a deterministic formula page for a local/remote TUI parity test.
    ///
    /// # Parameters
    /// - `self`: Query fixture.
    /// - `_context`: Application request context.
    /// - `request`: Exact project, snapshot, worksheet, range, and page bounds.
    ///
    /// # Returns
    /// One snapshot-bound formula cell.
    ///
    /// # Errors
    /// This fixture does not fail spreadsheet inspection.
    fn read_spreadsheet_cells(
        &self,
        _context: &ApplicationContext,
        request: &scriberig_app::SpreadsheetCellsQuery,
    ) -> Result<scriberig_app::SpreadsheetCells, ApplicationError> {
        let snapshot = request.snapshot;
        let workbook = EntityId::from_bytes([101; 16]);
        Ok(scriberig_app::SpreadsheetCells {
            project: request.project,
            snapshot,
            page: scriberig_project::SpreadsheetCellPage {
                snapshot,
                worksheet: request.worksheet,
                artifact_capability: None,
                artifact_diagnostic: None,
                cells: vec![scriberig_domain_spreadsheet::Cell {
                    id: scriberig_domain_spreadsheet::CellId::new(
                        workbook,
                        request.worksheet,
                        3,
                        2,
                    ),
                    worksheet: request.worksheet,
                    row: 3,
                    column: 2,
                    value: scriberig_domain_spreadsheet::CellValue::Formula(
                        scriberig_domain_spreadsheet::Formula {
                            expression: "=A3*2".to_owned(),
                            cached_value: Some(Box::new(
                                scriberig_domain_spreadsheet::CellValue::Number("20".to_owned()),
                            )),
                        },
                    ),
                    style: None,
                }],
                total: 1,
                has_more: false,
            },
        })
    }

    /// Returns a deterministic public inspection result.
    ///
    /// - `_context`: Application context for the inspection.
    /// - `request`: Inspection request to resolve.
    ///
    /// The deterministic inspection result.
    ///
    /// This fixture does not return an application error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(
        &self,
        _context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        Ok(InspectResult {
            entity: request.entity,
            revision: None,
            snapshot: match request.snapshot {
                scriberig_app::SnapshotResolution::At(snapshot) => Some(snapshot),
                _ => None,
            },
            object_type: None,
            label: Some("fixture".to_owned()),
            description: None,
            capabilities: Vec::new(),
            trust: None,
        })
    }
}

impl SessionBackend for Fixture {
    /// Rejects session reads in the query-only test fixture.
    ///
    /// - `_context`: Application context.
    /// - `_request`: Session read request.
    ///
    /// Never returns a session.
    ///
    /// Always returns an unsupported-operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read_session(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_app::SessionRequest,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "test session read".to_owned(),
        })
    }

    /// Returns a deterministic plan for shared adapter routing.
    ///
    /// - `_context`: Application request context.
    /// - `request`: Structured plan proposal.
    ///
    /// A valid plan carrying the requested goal and anchor.
    ///
    /// Returns an error when the fixture proposal cannot form a plan.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn create_plan(
        &self,
        _context: &ApplicationContext,
        request: &scriberig_app::PlanCreateRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        let mut plan = scriberig_session::Plan::new(
            EntityId::from_bytes([91; 16]),
            request.session,
            request.anchor,
            request.goal.clone(),
        )
        .map_err(|error| ApplicationError::InvalidRequest {
            operation: "fixture plan".to_owned(),
            message: format!("{error:?}"),
        })?;
        plan.transition(scriberig_session::PlanStatus::Planned)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "fixture plan".to_owned(),
                message: format!("{error:?}"),
            })?;
        Ok(plan)
    }

    /// Returns the fixture plan after receiving a shared node assignment.
    ///
    /// - `_context`: Application request context.
    /// - `request`: Selected plan-node assignment.
    ///
    /// The authoritative fixture plan.
    ///
    /// Returns an error if the fixture plan cannot be constructed.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_plan_node_assignment(
        &self,
        _context: &ApplicationContext,
        request: &scriberig_app::PlanAssignmentRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        let mut plan = scriberig_session::Plan::new(
            request.plan,
            request.session,
            scriberig_session::ContinuationAnchor::BranchHead {
                branch: EntityId::from_bytes([92; 16]),
            },
            "fixture assignment".to_owned(),
        )
        .map_err(|error| ApplicationError::InvalidRequest {
            operation: "fixture assignment".to_owned(),
            message: format!("{error:?}"),
        })?;
        plan.transition(scriberig_session::PlanStatus::Planned)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "fixture assignment".to_owned(),
                message: format!("{error:?}"),
            })?;
        Ok(plan)
    }

    /// Applies a fixture replan through the shared adapter contract.
    ///
    /// - `_context`: Application request context.
    /// - `request`: Complete structured revision proposal.
    ///
    /// The authoritative fixture plan.
    ///
    /// Returns an error if the fixture plan cannot be constructed.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn replan(
        &self,
        _context: &ApplicationContext,
        request: &scriberig_app::PlanReplanRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        let mut plan = scriberig_session::Plan::new(
            request.plan,
            request.session,
            scriberig_session::ContinuationAnchor::BranchHead {
                branch: EntityId::from_bytes([96; 16]),
            },
            "fixture replan".to_owned(),
        )
        .map_err(|error| ApplicationError::InvalidRequest {
            operation: "fixture replan".to_owned(),
            message: format!("{error:?}"),
        })?;
        plan.transition(scriberig_session::PlanStatus::Planned)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "fixture replan".to_owned(),
                message: format!("{error:?}"),
            })?;
        Ok(plan)
    }

    /// Rejects agent commands in the query-only test fixture.
    ///
    /// - `_context`: Application context.
    /// - `_request`: Agent command request.
    ///
    /// Never returns an operation identity.
    ///
    /// Always returns an unsupported-operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn submit_agent_command(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_app::AgentCommandRequest,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "test agent command".to_owned(),
        })
    }
}

impl scriberig_app::AgentConsoleBackend for Fixture {
    /// Rejects model-library reads in this query-only fixture.
    ///
    /// - `_context`: Application request context.
    ///
    /// Never returns a model library.
    ///
    /// Always returns an unsupported-operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn agent_model_library(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<rpi_model_runtime::SafeModelSnapshot>, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "test model library".to_owned(),
        })
    }

    /// Rejects model selection in this query-only fixture.
    ///
    /// - `_context`: Application request context.
    /// - `_request`: Requested session model selection.
    ///
    /// Never returns an updated selection.
    ///
    /// Always returns an unsupported-operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_session_model(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_app::SessionModelSelectionRequest,
    ) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "test model selection".to_owned(),
        })
    }

    /// Rejects thinking selection in this query-only fixture.
    ///
    /// - `_context`: Application request context.
    /// - `_request`: Requested session thinking selection.
    ///
    /// Never returns an updated selection.
    ///
    /// Always returns an unsupported-operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_session_thinking_level(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_app::SessionThinkingSelectionRequest,
    ) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "test thinking selection".to_owned(),
        })
    }
}

/// Verifies the adapter preserves exact public snapshot context.
#[test]
fn delegates_public_inspection_contract() {
    let adapter = ApplicationAdapter::new(Fixture, Vec::new());
    let entity = EntityId::from_bytes([7; 16]);
    let snapshot = SnapshotId::new([8; 32]);
    let result = adapter
        .inspect_entity(&ApplicationContext::empty(), entity, Some(snapshot))
        .expect("inspection");
    assert_eq!(result.entity, entity);
    assert_eq!(result.snapshot, Some(snapshot));
}

#[path = "service_tests_plan.rs"]
mod plan_tests;

#[path = "service_tests_artifact.rs"]
mod artifact_tests;
#[path = "service_tests_controls.rs"]
mod control_tests;
#[path = "service_tests_spreadsheet.rs"]
mod spreadsheet_tests;
