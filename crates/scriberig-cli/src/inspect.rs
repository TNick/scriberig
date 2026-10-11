//! Dispatches the CLI inspect command through the application query boundary.

use scriberig_app::{ApplicationContext, InspectResult, QueryService, SnapshotResolution};

use crate::InspectArgs;
use crate::trust::{TrustCommandError, inspect_at_with_format};

/// Executes one parsed CLI inspect request.
///
/// # Parameters
/// - `service`: Application query service supplied by composition.
/// - `context`: Caller and capability context.
/// - `args`: Parsed inspect command arguments.
///
/// # Returns
/// Human-readable or structured inspection output.
///
/// # Errors
/// Returns the structured trust command failure from the application boundary.
pub fn run<S: QueryService>(
    service: &S,
    context: &ApplicationContext,
    args: &InspectArgs,
) -> Result<String, TrustCommandError> {
    let snapshot = args
        .snapshot
        .map_or(SnapshotResolution::Current, SnapshotResolution::At);
    inspect_at_with_format(
        service,
        context,
        args.entity,
        snapshot,
        args.depth,
        args.format,
    )
}

/// Executes one parsed CLI inspect request and returns semantic data.
///
/// Product renderers should call this boundary when they need to compare or
/// further process inspection semantics without parsing terminal output.
///
/// # Parameters
/// - `service`: Application query service supplied by composition.
/// - `context`: Caller and capability context.
/// - `args`: Parsed inspect command arguments.
///
/// # Returns
/// Snapshot-bound semantic inspection result.
///
/// # Errors
/// Returns the structured trust command failure from the application boundary.
pub fn result<S: QueryService>(
    service: &S,
    context: &ApplicationContext,
    args: &InspectArgs,
) -> Result<InspectResult, TrustCommandError> {
    let snapshot = args
        .snapshot
        .map_or(SnapshotResolution::Current, SnapshotResolution::At);
    service
        .inspect(
            context,
            &scriberig_app::InspectRequest {
                entity: args.entity,
                snapshot,
                detail: match args.depth {
                    crate::trust::InspectDepth::Compact => scriberig_app::DetailLevel::Compact,
                    crate::trust::InspectDepth::Deep => scriberig_app::DetailLevel::Detailed,
                },
            },
        )
        .map_err(|error| match error {
            scriberig_app::ApplicationError::PermissionDenied { action, scope } => {
                TrustCommandError::AuthorizationDenied(scriberig_app::CapabilityDenial {
                    action,
                    scope,
                    reason: "application capability policy denied the request".to_owned(),
                    alternatives: Vec::new(),
                    approval_may_grant: false,
                })
            }
            other => TrustCommandError::Application(other),
        })
}
