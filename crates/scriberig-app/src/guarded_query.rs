//! Provides one application authorization boundary for universal read queries.

use crate::guarded_query_support::{
    filter_find_hits, filter_navigation, filter_query_rows, query_error_to_application,
    target_entity, target_snapshot,
};
use crate::{
    ApplicationContext, ApplicationError, CapabilityAction, CapabilityScope, InspectRequest,
    InspectResult, OperationKind, QueryService,
};
use scriberig_query::{
    FindRequest, FindResult, FindService, FollowRequest, FollowResult, InspectService, Inspection,
    ListRequest, ListResult, QueryResult, QuerySpec, StructuredQueryService, TraverseService,
};

/// Wraps reusable read providers with application capability enforcement.
pub struct GuardedQueryService<S> {
    /// Policy-free provider used only after request-level authorization.
    inner: S,
}

impl<S> GuardedQueryService<S> {
    /// Creates a guarded application read service.
    ///
    /// # Parameters
    /// - `inner`: Reusable provider that supplies semantic read operations.
    ///
    /// # Returns
    /// A provider whose exposed results are checked against request grants.
    #[must_use]
    pub const fn new(inner: S) -> Self {
        Self { inner }
    }

    /// Borrows the wrapped reusable provider for intentional low-level use.
    ///
    /// # Returns
    /// The policy-free provider.
    #[must_use]
    pub const fn inner(&self) -> &S {
        &self.inner
    }

    /// Returns a structured denial without including hidden content.
    ///
    /// # Parameters
    /// - `action`: Action that the caller requested.
    /// - `scope`: Scope that the caller requested.
    ///
    /// # Returns
    /// A safe application permission error.
    fn denied(action: &CapabilityAction, scope: &CapabilityScope) -> ApplicationError {
        ApplicationError::PermissionDenied {
            action: format!("{action:?}"),
            scope: format!("{scope:?}"),
        }
    }

    /// Checks one operation-level capability.
    ///
    /// # Parameters
    /// - `context`: Caller grants to evaluate.
    /// - `action`: Action required by the operation.
    /// - `scope`: Semantic scope exposed by the operation.
    /// - `snapshot`: Exact snapshot constraint, when available.
    /// - `operation`: Universal operation kind.
    ///
    /// # Returns
    /// `Ok(())` when one grant covers the request.
    ///
    /// # Errors
    /// Returns a structured permission denial when no grant covers it.
    fn authorize(
        context: &ApplicationContext,
        action: &CapabilityAction,
        scope: &CapabilityScope,
        snapshot: Option<scriberig_core::SnapshotId>,
        operation: OperationKind,
    ) -> Result<(), ApplicationError> {
        if context
            .capabilities
            .permits(action, scope, snapshot, Some(operation))
        {
            Ok(())
        } else {
            Err(Self::denied(action, scope))
        }
    }
}

impl<S: QueryService> QueryService for GuardedQueryService<S> {
    /// Authorizes and delegates one application inspection.
    ///
    /// # Parameters
    /// - `self`: Guarded provider.
    /// - `context`: Caller grants and exact request context.
    /// - `request`: Entity inspection request.
    ///
    /// # Returns
    /// The provider's inspection result after authorization.
    ///
    /// # Errors
    /// Returns a structured permission denial or provider error.
    fn inspect(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        let scope = CapabilityScope::Entity(request.entity);
        Self::authorize(
            context,
            &CapabilityAction::Inspect,
            &scope,
            match request.snapshot {
                crate::SnapshotResolution::At(snapshot) => Some(snapshot),
                _ => context.snapshot,
            },
            crate::OperationKind::Inspect,
        )?;
        let result = self.inner.inspect(context, request)?;
        Ok(result)
    }
}

impl<S: InspectService> GuardedQueryService<S> {
    /// Authorizes and delegates a reusable exact-target inspection.
    ///
    /// # Parameters
    /// - `self`: Guarded provider.
    /// - `context`: Caller grants and exact request context.
    /// - `request`: Query-crate inspection request.
    ///
    /// # Returns
    /// The inspection result when its entity is authorized.
    ///
    /// # Errors
    /// Returns a structured permission denial or provider error.
    pub fn inspect_target(
        &self,
        context: &ApplicationContext,
        request: &scriberig_query::InspectRequest,
    ) -> Result<Inspection, ApplicationError> {
        let entity = target_entity(request.target);
        let scope = CapabilityScope::Entity(entity);
        Self::authorize(
            context,
            &CapabilityAction::Inspect,
            &scope,
            target_snapshot(request.target),
            crate::OperationKind::Inspect,
        )?;
        if let Some(view) = &request.view {
            let projection = CapabilityScope::Projection {
                entity,
                projection: view.0.clone(),
            };
            Self::authorize(
                context,
                &CapabilityAction::Render,
                &projection,
                target_snapshot(request.target),
                OperationKind::Render,
            )?;
        }
        self.inner
            .inspect(request)
            .map_err(query_error_to_application)
    }
}

impl<S: TraverseService> GuardedQueryService<S> {
    /// Authorizes collection access and filters restricted entries.
    ///
    /// # Parameters
    /// - `self`: Guarded provider.
    /// - `context`: Caller grants and exact request context.
    /// - `request`: Collection listing request.
    ///
    /// # Returns
    /// A page containing only authorized navigation entries.
    ///
    /// # Errors
    /// Returns a structured denial or provider error.
    pub fn list(
        &self,
        context: &ApplicationContext,
        request: &ListRequest,
    ) -> Result<ListResult, ApplicationError> {
        let scope = CapabilityScope::Entity(request.subject.entity);
        Self::authorize(
            context,
            &CapabilityAction::Query,
            &scope,
            Some(request.subject.snapshot),
            crate::OperationKind::List,
        )?;
        let mut result = self
            .inner
            .list(request)
            .map_err(query_error_to_application)?;
        filter_navigation(context, &mut result.page.items);
        Ok(result)
    }

    /// Authorizes relation access and filters restricted entries.
    ///
    /// # Parameters
    /// - `self`: Guarded provider.
    /// - `context`: Caller grants and exact request context.
    /// - `request`: Relation traversal request.
    ///
    /// # Returns
    /// A page containing only authorized navigation entries.
    ///
    /// # Errors
    /// Returns a structured denial or provider error.
    pub fn follow(
        &self,
        context: &ApplicationContext,
        request: &FollowRequest,
    ) -> Result<FollowResult, ApplicationError> {
        let scope = CapabilityScope::Relation {
            entity: request.subject.entity,
            relation: request.relation.relation_type.clone(),
        };
        Self::authorize(
            context,
            &CapabilityAction::Query,
            &scope,
            Some(request.subject.snapshot),
            crate::OperationKind::Follow,
        )?;
        let mut result = self
            .inner
            .follow(request)
            .map_err(query_error_to_application)?;
        filter_navigation(context, &mut result.page.items);
        Ok(result)
    }
}

impl<S: FindService> GuardedQueryService<S> {
    /// Authorizes discovery and filters restricted hits and previews.
    ///
    /// # Parameters
    /// - `self`: Guarded provider.
    /// - `context`: Caller grants and exact request context.
    /// - `request`: Exact-scope discovery request.
    ///
    /// # Returns
    /// A page containing only authorized semantic hits.
    ///
    /// # Errors
    /// Returns a structured denial or provider error.
    pub fn find(
        &self,
        context: &ApplicationContext,
        request: &FindRequest,
    ) -> Result<FindResult, ApplicationError> {
        authorize_query_scope(
            context,
            &request.scope,
            &CapabilityAction::Query,
            OperationKind::Find,
        )?;
        let mut result = self
            .inner
            .find(request)
            .map_err(query_error_to_application)?;
        filter_find_hits(context, &mut result);
        Ok(result)
    }
}

impl<S: StructuredQueryService> GuardedQueryService<S> {
    /// Authorizes exact structural queries and filters restricted rows.
    ///
    /// # Parameters
    /// - `self`: Guarded provider.
    /// - `context`: Caller grants and exact request context.
    /// - `request`: Exact-scope structural query.
    ///
    /// # Returns
    /// A page containing only authorized semantic rows.
    ///
    /// # Errors
    /// Returns a structured denial or provider error.
    pub fn query(
        &self,
        context: &ApplicationContext,
        request: &QuerySpec,
    ) -> Result<QueryResult, ApplicationError> {
        authorize_query_scope(
            context,
            &request.scope,
            &CapabilityAction::Query,
            OperationKind::Query,
        )?;
        let mut result = self
            .inner
            .query(request)
            .map_err(query_error_to_application)?;
        filter_query_rows(context, &mut result);
        Ok(result)
    }
}

/// Authorizes every entity named by a query scope before evaluation.
///
/// # Parameters
/// - `context`: Caller grants to evaluate.
/// - `scope`: Exact query scope to authorize.
/// - `action`: Action required by the query.
/// - `operation`: Universal operation kind.
///
/// # Returns
/// `Ok(())` when the complete scope is authorized.
///
/// # Errors
/// Returns a denial or schema error before the provider is evaluated.
fn authorize_query_scope(
    context: &ApplicationContext,
    scope: &scriberig_query::QueryScope,
    action: &CapabilityAction,
    operation: OperationKind,
) -> Result<(), ApplicationError> {
    match scope {
        scriberig_query::QueryScope::Snapshot(snapshot) => GuardedQueryService::<()>::authorize(
            context,
            action,
            &CapabilityScope::Project,
            Some(*snapshot),
            operation,
        ),
        scriberig_query::QueryScope::Entity { snapshot, entity } => {
            GuardedQueryService::<()>::authorize(
                context,
                action,
                &CapabilityScope::Entity(*entity),
                Some(*snapshot),
                operation,
            )
        }
        scriberig_query::QueryScope::ExplicitEntities { snapshot, entities } => {
            if entities.is_empty() {
                return Err(ApplicationError::SchemaViolation {
                    message: "query scope must contain an entity".to_owned(),
                });
            }
            for entity in entities {
                GuardedQueryService::<()>::authorize(
                    context,
                    action,
                    &CapabilityScope::Entity(*entity),
                    Some(*snapshot),
                    operation,
                )?;
            }
            Ok(())
        }
    }
}
