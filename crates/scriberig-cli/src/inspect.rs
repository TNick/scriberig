//! Dispatches the CLI inspect command through the application query boundary.

use scriberig_app::{ApplicationContext, QueryService, SnapshotResolution};

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
