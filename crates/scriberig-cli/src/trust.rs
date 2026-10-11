//! Renders shared trust summaries for human-readable CLI inspection.

use std::fmt::Write;

use scriberig_app::{
    ApplicationContext, ApplicationError, CapabilityDenial, DetailLevel, InspectRequest,
    InspectResult, QueryService, SnapshotResolution, SourceKind, ValidationState,
};
use scriberig_core::EntityId;

/// Selects the amount of trust metadata printed by inspection.
#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum InspectDepth {
    /// Print the compact provenance and validation summary.
    Compact,
    /// Include stable handles for deeper provenance inspection.
    Deep,
}

/// Exit status used when application authorization denies an operation.
pub const AUTHORIZATION_DENIED_EXIT_CODE: i32 = 77;

/// Reports trust-inspection failures without flattening authorization denials.
#[derive(Debug)]
pub enum TrustCommandError {
    /// The application denied the requested capability.
    AuthorizationDenied(CapabilityDenial),
    /// The application returned another structured failure.
    Application(ApplicationError),
}

impl TrustCommandError {
    /// Returns the process exit status for the failure.
    ///
    /// # Parameters
    /// - `self`: Trust command failure.
    ///
    /// # Returns
    /// Distinct authorization status or the generic application failure status.
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        match self {
            Self::AuthorizationDenied(_) => AUTHORIZATION_DENIED_EXIT_CODE,
            Self::Application(_) => 1,
        }
    }
}

/// Executes an entity inspection through the shared application query boundary.
///
/// # Parameters
/// - `service`: Application query service selected by the product composition root.
/// - `context`: Caller and capability context for the request.
/// - `entity`: Entity identifier to inspect.
/// - `depth`: Compact or deep trust view.
///
/// # Returns
/// Rendered trust inspection output.
///
/// # Errors
/// Returns the structured application error returned by the query service.
pub fn inspect<S: QueryService>(
    service: &S,
    context: &ApplicationContext,
    entity: EntityId,
    depth: InspectDepth,
) -> Result<String, ApplicationError> {
    inspect_at(service, context, entity, SnapshotResolution::Current, depth).map_err(|error| {
        match error {
            TrustCommandError::Application(error) => error,
            TrustCommandError::AuthorizationDenied(denial) => ApplicationError::PermissionDenied {
                action: denial.action,
                scope: denial.scope,
            },
        }
    })
}

/// Executes inspection with an explicit snapshot resolution.
///
/// # Parameters
/// - `service`: Application query service selected by the product composition root.
/// - `context`: Caller and capability context for the request.
/// - `entity`: Entity identifier to inspect.
/// - `snapshot`: Snapshot resolution required by the caller.
/// - `depth`: Compact or deep trust view.
///
/// # Returns
/// Rendered trust inspection output.
///
/// # Errors
/// Returns a structured application failure or a distinct authorization denial.
pub fn inspect_at<S: QueryService>(
    service: &S,
    context: &ApplicationContext,
    entity: EntityId,
    snapshot: SnapshotResolution,
    depth: InspectDepth,
) -> Result<String, TrustCommandError> {
    let request = InspectRequest {
        entity,
        snapshot,
        detail: match depth {
            InspectDepth::Compact => DetailLevel::Compact,
            InspectDepth::Deep => DetailLevel::Detailed,
        },
    };
    service.inspect(context, &request).map_or_else(
        |error| match error {
            ApplicationError::PermissionDenied { action, scope } => {
                Err(TrustCommandError::AuthorizationDenied(CapabilityDenial {
                    action,
                    scope,
                    reason: "application capability policy denied the request".to_owned(),
                    alternatives: Vec::new(),
                    approval_may_grant: false,
                }))
            }
            other => Err(TrustCommandError::Application(other)),
        },
        |result| Ok(render_inspection(&result, depth)),
    )
}

/// Renders an inspection result without exposing object content or secrets.
///
/// # Parameters
/// - `result`: Shared application inspection result.
/// - `depth`: Compact or deep trust view.
///
/// # Returns
/// Human-readable inspection output.
#[must_use]
pub fn render_inspection(result: &InspectResult, depth: InspectDepth) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "entity: {}", result.entity);
    if let Some(snapshot) = result.snapshot {
        let _ = writeln!(output, "snapshot: {snapshot}");
    }
    if let Some(revision) = result.revision {
        let _ = writeln!(output, "revision: {revision}");
    }
    if let Some(trust) = &result.trust {
        let _ = writeln!(output, "source: {}", source_label(&trust.source));
        let _ = writeln!(
            output,
            "validation: {}",
            validation_label(&trust.validation)
        );
        if let Some(accepted_via) = &trust.accepted_via {
            let _ = writeln!(output, "accepted via: {accepted_via}");
        }
        if depth == InspectDepth::Deep {
            for handle in &trust.provenance {
                let _ = writeln!(output, "provenance: {handle}");
            }
            if let Some(policy) = &trust.policy {
                let _ = writeln!(output, "policy: {:?}", policy.decision);
                if let Some(handle) = &policy.handle {
                    let _ = writeln!(output, "policy evidence: {handle}");
                }
            }
            if let Some(approval) = &trust.approval {
                let _ = writeln!(output, "approval required: {}", approval.required);
                let _ = writeln!(output, "approval granted: {}", approval.granted);
                if let Some(handle) = &approval.handle {
                    let _ = writeln!(output, "approval evidence: {handle}");
                }
            }
            if let Some(conflict) = &trust.conflict {
                let _ = writeln!(output, "conflict: {}", conflict.handle);
                let _ = writeln!(output, "conflict resolvable: {}", conflict.resolvable);
            }
            if let Some(effect) = &trust.external_effect {
                let _ = writeln!(
                    output,
                    "external effect: {} ({})",
                    effect.handle, effect.state
                );
            }
        }
    }
    output
}

/// Renders a structured capability denial without restricted object content.
///
/// # Parameters
/// - `denial`: Shared application denial details.
///
/// # Returns
/// Human-readable denial output.
#[must_use]
pub fn render_denial(denial: &CapabilityDenial) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "denied action: {}", denial.action);
    let _ = writeln!(output, "requested scope: {}", denial.scope);
    let _ = writeln!(output, "reason: {}", denial.reason);
    let _ = writeln!(output, "approval may grant: {}", denial.approval_may_grant);
    for alternative in &denial.alternatives {
        let _ = writeln!(output, "alternative: {alternative}");
    }
    output
}

/// Returns the stable display label for a source kind.
///
/// # Parameters
/// - `source`: Source kind to label.
///
/// # Returns
/// Stable machine-readable source label.
fn source_label(source: &SourceKind) -> &'static str {
    match source {
        SourceKind::Authored => "authored",
        SourceKind::Observed => "observed",
        SourceKind::Imported => "imported",
        SourceKind::Derived => "derived",
        SourceKind::ModelGenerated => "model_generated",
    }
}

/// Returns the stable display label for a validation state.
///
/// # Parameters
/// - `validation`: Validation state to label.
///
/// # Returns
/// Stable machine-readable validation label.
fn validation_label(validation: &ValidationState) -> &'static str {
    match validation {
        ValidationState::Current => "current",
        ValidationState::Stale => "stale",
        ValidationState::Failed => "failed",
        ValidationState::Unknown => "unknown",
    }
}

#[cfg(test)]
#[path = "trust_tests.rs"]
mod tests;
