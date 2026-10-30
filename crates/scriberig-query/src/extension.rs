//! Registers strongly typed domain query capabilities behind stable identifiers.

use std::collections::BTreeMap;

use scriberig_core::SnapshotId;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{QueryError, ReadOperation};

/// Defines one domain-owned query request and response contract.
///
/// Each implementation owns concrete serializable request and response types.
/// The registry uses a private serialization bridge for dispatch; callers
/// always submit and receive those concrete types.
pub trait DomainQueryExtension: Send + Sync + 'static {
    /// Concrete request accepted by this extension.
    type Request: Serialize + DeserializeOwned;
    /// Concrete response returned by this extension.
    type Response: Serialize + DeserializeOwned;

    /// Stable, namespaced query identifier.
    const ID: &'static str;

    /// Executes the query against its exact snapshot.
    ///
    /// # Parameters
    /// - `self`: Registered domain implementation.
    /// - `snapshot`: Exact snapshot selected by the caller.
    /// - `request`: Validated extension-specific request.
    ///
    /// # Returns
    /// The extension-specific typed response.
    ///
    /// # Errors
    /// Returns a semantic query error when the snapshot or request cannot be served.
    fn execute(
        &self,
        snapshot: SnapshotId,
        request: Self::Request,
    ) -> Result<Self::Response, QueryError>;
}

/// Provides a typed response bound to the exact snapshot that produced it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomainQueryResult<Response> {
    /// Exact snapshot used by the extension.
    pub snapshot: SnapshotId,
    /// Concrete response type declared by the extension.
    pub response: Response,
}

/// Stores domain query extensions by stable identifier.
#[derive(Default)]
pub struct DomainQueryRegistry {
    extensions: BTreeMap<&'static str, Box<dyn ErasedExtension>>,
}

impl DomainQueryRegistry {
    /// Creates an empty domain query registry.
    ///
    /// # Returns
    /// Registry with no installed extensions.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Installs one typed query extension under its stable identifier.
    ///
    /// # Parameters
    /// - `extension`: Domain-owned implementation to register.
    ///
    /// # Returns
    /// Unit when registration succeeds.
    ///
    /// # Errors
    /// Returns an error for an invalid or duplicate extension identifier.
    pub fn register<E>(&mut self, extension: E) -> Result<(), QueryError>
    where
        E: DomainQueryExtension,
    {
        if !valid_extension_id(E::ID) {
            return Err(QueryError::InvalidQuery {
                message: format!("invalid domain query extension identifier {:?}", E::ID),
            });
        }
        if self.extensions.contains_key(E::ID) {
            return Err(QueryError::InvalidQuery {
                message: format!("domain query extension {:?} is already registered", E::ID),
            });
        }
        self.extensions
            .insert(E::ID, Box::new(TypedExtension(extension)));
        Ok(())
    }

    /// Executes a registered extension through its concrete request/response types.
    ///
    /// # Parameters
    /// - `snapshot`: Exact snapshot to bind to the operation.
    /// - `request`: Concrete request type associated with extension `E`.
    ///
    /// # Returns
    /// Typed response and the exact snapshot used to produce it.
    ///
    /// # Errors
    /// Returns an error when the extension is absent, serialization fails, or execution fails.
    pub fn query<E>(
        &self,
        snapshot: SnapshotId,
        request: &E::Request,
    ) -> Result<DomainQueryResult<E::Response>, QueryError>
    where
        E: DomainQueryExtension,
    {
        let extension =
            self.extensions
                .get(E::ID)
                .ok_or_else(|| QueryError::UnsupportedCapability {
                    operation: ReadOperation::Query,
                    capability: E::ID.to_owned(),
                })?;
        if extension.request_type_name() != std::any::type_name::<E::Request>()
            || extension.response_type_name() != std::any::type_name::<E::Response>()
        {
            return Err(QueryError::InvalidQuery {
                message: format!(
                    "domain query identifier {:?} is registered with a different typed contract",
                    E::ID
                ),
            });
        }
        let request = serde_json::to_value(request).map_err(|error| QueryError::InvalidQuery {
            message: format!("domain query request cannot be encoded: {error}"),
        })?;
        let response = extension.execute(snapshot, request)?;
        let response = serde_json::from_value(response).map_err(|error| QueryError::Internal {
            operation: ReadOperation::Query,
            message: format!("domain query response did not match its typed contract: {error}"),
        })?;
        Ok(DomainQueryResult { snapshot, response })
    }
}

/// Defines the private serialized dispatch boundary for installed extensions.
trait ErasedExtension: Send + Sync {
    /// Returns the concrete registered request type.
    ///
    /// # Returns
    /// Fully qualified Rust request type name.
    fn request_type_name(&self) -> &'static str;

    /// Returns the concrete registered response type.
    ///
    /// # Returns
    /// Fully qualified Rust response type name.
    fn response_type_name(&self) -> &'static str;

    /// Decodes one concrete request, executes the extension, and encodes its result.
    ///
    /// # Parameters
    /// - `self`: Installed extension implementation.
    /// - `snapshot`: Exact query snapshot.
    /// - `request`: Serialized concrete request.
    ///
    /// # Returns
    /// Serialized concrete response.
    ///
    /// # Errors
    /// Returns decode or extension execution failures.
    fn execute(
        &self,
        snapshot: SnapshotId,
        request: serde_json::Value,
    ) -> Result<serde_json::Value, QueryError>;
}

/// Wraps one concrete extension for private registry dispatch.
struct TypedExtension<E>(E);

impl<E> ErasedExtension for TypedExtension<E>
where
    E: DomainQueryExtension,
{
    /// Reports the request contract declared by the typed extension.
    ///
    /// # Returns
    /// Fully qualified Rust request type name.
    fn request_type_name(&self) -> &'static str {
        std::any::type_name::<E::Request>()
    }

    /// Reports the response contract declared by the typed extension.
    ///
    /// # Returns
    /// Fully qualified Rust response type name.
    fn response_type_name(&self) -> &'static str {
        std::any::type_name::<E::Response>()
    }

    /// Dispatches the request through the extension's declared concrete types.
    ///
    /// # Parameters
    /// - `self`: Typed extension wrapper.
    /// - `snapshot`: Exact query snapshot.
    /// - `request`: Serialized extension-specific request.
    ///
    /// # Returns
    /// Serialized extension-specific response.
    ///
    /// # Errors
    /// Returns a query error for invalid request data or extension failure.
    fn execute(
        &self,
        snapshot: SnapshotId,
        request: serde_json::Value,
    ) -> Result<serde_json::Value, QueryError> {
        let request = serde_json::from_value::<E::Request>(request).map_err(|error| {
            QueryError::InvalidQuery {
                message: format!("domain query request does not match its typed contract: {error}"),
            }
        })?;
        let response = self.0.execute(snapshot, request)?;
        serde_json::to_value(response).map_err(|error| QueryError::Internal {
            operation: ReadOperation::Query,
            message: format!("domain query response cannot be encoded: {error}"),
        })
    }
}

/// Checks the namespaced lowercase identifier format used for extensions.
///
/// # Parameters
/// - `identifier`: Stable query extension identifier.
///
/// # Returns
/// True when the identifier contains at least one namespace separator and only safe characters.
fn valid_extension_id(identifier: &str) -> bool {
    identifier.contains('.')
        && identifier.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'.' || byte == b'_'
        })
        && identifier
            .split('.')
            .all(|segment| !segment.is_empty() && segment.as_bytes()[0].is_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    //! Verifies registered domain query extensions remain typed and snapshot-bound.

    use serde::{Deserialize, Serialize};

    use super::*;

    /// Defines the concrete fixture request for the registered query.
    #[derive(Deserialize, Serialize)]
    struct FixtureRequest {
        /// Attribute filter requested by the caller.
        attribute: String,
    }

    /// Defines the concrete fixture response for the registered query.
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct FixtureResponse {
        /// Stable handles returned from the requested snapshot.
        feature_ids: Vec<String>,
    }

    /// Implements the fixture's typed attribute selector.
    struct FixtureSelector;

    impl DomainQueryExtension for FixtureSelector {
        type Request = FixtureRequest;
        type Response = FixtureResponse;

        const ID: &'static str = "geo.features_by_attribute";

        /// Returns fixture results that demonstrate exact snapshot propagation.
        ///
        /// # Parameters
        /// - `self`: Fixture selector implementation.
        /// - `snapshot`: Exact test snapshot.
        /// - `request`: Typed attribute query.
        ///
        /// # Returns
        /// One deterministic feature handle for a nonempty attribute name.
        ///
        /// # Errors
        /// Returns an invalid-query error when the requested attribute is blank.
        fn execute(
            &self,
            snapshot: SnapshotId,
            request: Self::Request,
        ) -> Result<Self::Response, QueryError> {
            if request.attribute.is_empty() {
                return Err(QueryError::InvalidQuery {
                    message: "attribute name cannot be empty".to_owned(),
                });
            }
            Ok(FixtureResponse {
                feature_ids: vec![format!("{}:{}", snapshot, request.attribute)],
            })
        }
    }

    /// Provides a conflicting Rust type for the same stable extension identifier.
    struct FixtureAlias;

    /// Request payload owned by the conflicting test extension.
    #[derive(Deserialize, Serialize)]
    struct AliasRequest {
        /// Text selected for the test query.
        attribute: String,
    }

    /// Response payload owned by the conflicting test extension.
    #[derive(Deserialize, Eq, PartialEq, Serialize)]
    struct AliasResponse {
        /// Test feature identities.
        feature_ids: Vec<String>,
    }

    impl DomainQueryExtension for FixtureAlias {
        type Request = AliasRequest;
        type Response = AliasResponse;

        const ID: &'static str = FixtureSelector::ID;

        /// Returns a response when called directly by the registry.
        ///
        /// # Parameters
        /// - `self`: Conflicting extension implementation.
        /// - `snapshot`: Exact test snapshot.
        /// - `request`: Alias request payload.
        ///
        /// # Returns
        /// One deterministic test feature identity.
        ///
        /// # Errors
        /// This fixture does not fail.
        fn execute(
            &self,
            snapshot: SnapshotId,
            request: Self::Request,
        ) -> Result<Self::Response, QueryError> {
            Ok(AliasResponse {
                feature_ids: vec![format!("{snapshot}:{}", request.attribute)],
            })
        }
    }

    /// Checks that typed requests return exact-snapshot-tagged results.
    #[test]
    fn registered_extension_preserves_typed_request_and_snapshot() {
        let snapshot = SnapshotId::new([42; 32]);
        let mut registry = DomainQueryRegistry::new();
        registry
            .register(FixtureSelector)
            .expect("register typed query extension");
        let result = registry
            .query::<FixtureSelector>(
                snapshot,
                &FixtureRequest {
                    attribute: "road_class".to_owned(),
                },
            )
            .expect("execute typed query");
        assert_eq!(result.snapshot, snapshot);
        assert_eq!(
            result.response.feature_ids,
            vec![format!("{snapshot}:road_class")]
        );
    }

    /// Checks duplicate identifiers are rejected instead of shadowing providers.
    #[test]
    fn duplicate_extension_registration_is_rejected() {
        let mut registry = DomainQueryRegistry::new();
        registry
            .register(FixtureSelector)
            .expect("first registration");
        assert!(registry.register(FixtureSelector).is_err());
    }

    /// Checks a shared ID cannot dispatch between incompatible typed contracts.
    #[test]
    fn extension_identifier_requires_the_registered_typed_contract() {
        let mut registry = DomainQueryRegistry::new();
        registry
            .register(FixtureSelector)
            .expect("register selector");
        let result = registry.query::<FixtureAlias>(
            SnapshotId::new([43; 32]),
            &AliasRequest {
                attribute: "road_class".to_owned(),
            },
        );
        assert!(matches!(result, Err(QueryError::InvalidQuery { .. })));
    }
}
