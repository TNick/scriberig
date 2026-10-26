//! Binds model connections to host-authorized credential references.

use std::sync::Arc;

use rpi_ai::Model;
use scriberig_secrets::{ExecutionMode, SecretRef, SecretUseContext};

use super::AuthorizedCredentialProvider;

/// Resolves the optional host credential binding for an exact model connection.
///
/// The environment value is a JSON array of objects with `provider_id`,
/// `model_id`, and `credential` fields. The endpoint and project scope always
/// come from the selected upstream model and current project.
///
/// # Parameters
/// - `model`: Exact model selected from the upstream catalog.
///
/// # Returns
/// The per-connection credential adapter, or `None` when no binding matches.
///
/// # Errors
/// Returns a safe configuration error for malformed or ambiguous bindings.
pub(crate) fn authorized_connection_credential_provider(
    model: &Model,
) -> Result<Option<AuthorizedCredentialProvider>, String> {
    let Ok(raw) = std::env::var("SCRIBE_AGENT_CONNECTION_CREDENTIALS") else {
        return Ok(None);
    };
    authorized_connection_credential_provider_from(&raw, model)
}

/// Resolves one exact model binding from a supplied host configuration document.
///
/// # Parameters
/// - `raw`: JSON document read from host process configuration.
/// - `model`: Exact model selected from the upstream catalog.
///
/// # Returns
/// The per-connection credential adapter, or `None` when no binding matches.
///
/// # Errors
/// Returns a safe configuration error for malformed or ambiguous bindings.
fn authorized_connection_credential_provider_from(
    raw: &str,
    model: &Model,
) -> Result<Option<AuthorizedCredentialProvider>, String> {
    let project_root = std::env::current_dir()
        .and_then(|root| root.canonicalize())
        .map_err(|_| "invalid provider credential project scope".to_owned())?;
    let locations = scriberig_secrets::registry::RegistryLocations::from_process()
        .map_err(|_| "invalid provider credential policy location".to_owned())?;
    authorized_connection_credential_provider_in(raw, model, project_root, locations)
}

/// Builds the selected provider adapter with explicit trusted host authority.
///
/// # Parameters
/// - `raw`: Validated-host configuration document.
/// - `model`: Exact upstream model selected for execution.
/// - `project_root`: Canonical project root supplied by the composition root.
/// - `locations`: User and administrator secret-policy locations.
///
/// # Returns
/// The selected credential adapter, or `None` without an exact connection row.
///
/// # Errors
/// Returns a safe configuration error for invalid or ambiguous bindings.
fn authorized_connection_credential_provider_in(
    raw: &str,
    model: &Model,
    project_root: std::path::PathBuf,
    locations: scriberig_secrets::registry::RegistryLocations,
) -> Result<Option<AuthorizedCredentialProvider>, String> {
    let bindings = parse_bindings(raw)?;
    let Some(binding) = matching_binding(bindings, &model.provider, &model.id)? else {
        return Ok(None);
    };
    let context = SecretUseContext {
        project_root,
        purpose: "model_provider".to_owned(),
        mode: ExecutionMode::Interactive,
    };
    let provider = scriberig_secrets::registry::AuthorizedCredential {
        reference: binding.credential,
        context,
        locations,
        invocation_binding: None,
        resolver: Arc::new(scriberig_secrets::SourceResolver),
    };
    Ok(Some(Arc::new(provider)))
}

/// Selects the unique binding for an exact provider/model pair.
///
/// # Parameters
/// - `bindings`: Validated host connection bindings.
/// - `provider_id`: Upstream provider identifier.
/// - `model_id`: Upstream model identifier.
///
/// # Returns
/// The matching binding, or `None` when the pair is not configured.
///
/// # Errors
/// Returns an error when multiple entries claim the same connection.
fn matching_binding(
    bindings: Vec<ConnectionCredentialBinding>,
    provider_id: &str,
    model_id: &str,
) -> Result<Option<ConnectionCredentialBinding>, String> {
    let mut matching = bindings
        .into_iter()
        .filter(|binding| binding.provider_id == provider_id && binding.model_id == model_id);
    let selected = matching.next();
    if matching.next().is_some() {
        return Err("ambiguous provider connection credential binding".to_owned());
    }
    Ok(selected)
}

/// Parses and validates the host-owned connection binding document.
///
/// # Parameters
/// - `raw`: JSON document read from host process configuration.
///
/// # Returns
/// Validated, indirect credential bindings.
///
/// # Errors
/// Returns a generic error without echoing configuration values.
fn parse_bindings(raw: &str) -> Result<Vec<ConnectionCredentialBinding>, String> {
    let bindings: Vec<ConnectionCredentialBinding> = serde_json::from_str(raw)
        .map_err(|_| "invalid provider connection credentials".to_owned())?;
    for binding in &bindings {
        if binding.provider_id.trim().is_empty()
            || binding.model_id.trim().is_empty()
            || binding.credential.validate().is_err()
        {
            return Err("invalid provider connection credentials".to_owned());
        }
    }
    Ok(bindings)
}

/// Identifies one exact model connection and an indirect secret reference.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectionCredentialBinding {
    /// Exact public provider identifier.
    provider_id: String,
    /// Exact public model identifier.
    model_id: String,
    /// Host-selected secret source, never a secret value.
    credential: SecretRef,
}

#[cfg(test)]
mod tests {
    //! Covers strict parsing of host-owned connection credentials.

    use super::{authorized_connection_credential_provider_in, matching_binding, parse_bindings};
    use rpi_ai::Provider;
    use scriberig_secrets::{Binding, ExecutionMode, SecretRef};
    use std::io::{Read, Write};
    use std::thread::JoinHandle;

    /// Rejects unknown configuration fields without echoing input values.
    #[test]
    fn parses_exact_indirect_bindings_and_rejects_unknown_fields() {
        let valid = r#"[{"provider_id":"vendor","model_id":"model-a","credential":{"source":"environment","name":"VENDOR_TOKEN"}}]"#;
        assert_eq!(parse_bindings(valid).unwrap().len(), 1);
        let invalid = r#"[{"provider_id":"vendor","model_id":"model-a","credential":{"source":"environment","name":"VENDOR_TOKEN"},"base_url":"https://attacker.invalid"}]"#;
        assert!(parse_bindings(invalid).is_err());
    }

    /// Selects credentials only for the exact upstream provider/model pair.
    #[test]
    fn connection_reference_is_bound_to_the_selected_model() {
        let raw = r#"[{"provider_id":"vendor","model_id":"model-a","credential":{"source":"environment","name":"VENDOR_TOKEN"}},{"provider_id":"vendor","model_id":"model-b","credential":{"source":"environment","name":"OTHER_TOKEN"}}]"#;
        let bindings = parse_bindings(raw).unwrap();
        assert_eq!(
            matching_binding(bindings, "vendor", "model-a")
                .unwrap()
                .map(|binding| binding.credential),
            Some(SecretRef::Environment {
                name: "VENDOR_TOKEN".to_owned()
            })
        );
        assert!(
            matching_binding(parse_bindings(raw).unwrap(), "other", "model-a")
                .unwrap()
                .is_none()
        );
    }

    /// Rejects duplicate credentials for one provider/model connection.
    #[test]
    fn duplicate_connection_bindings_fail_closed() {
        let raw = r#"[{"provider_id":"vendor","model_id":"model-a","credential":{"source":"environment","name":"ONE_TOKEN"}},{"provider_id":"vendor","model_id":"model-a","credential":{"source":"environment","name":"TWO_TOKEN"}}]"#;
        assert!(matching_binding(parse_bindings(raw).unwrap(), "vendor", "model-a").is_err());
    }

    #[expect(
        clippy::too_many_lines,
        reason = "credential authorization regression covers the full protected file flow"
    )]
    #[test]
    /// Builds an upstream request secret only for the selected authorized endpoint.
    fn selected_connection_produces_authorized_request_secret() {
        let temporary = tempfile::tempdir().expect("temporary root");
        let project_root = temporary.path().canonicalize().expect("canonical project");
        let (endpoint, provider_server) = responses_fixture();
        let credential_path = project_root.join("provider-token");
        std::fs::write(&credential_path, "connection-secret-test-only")
            .expect("write temporary credential");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&credential_path, std::fs::Permissions::from_mode(0o600))
                .expect("protect temporary credential");
        }
        let reference = SecretRef::File {
            path: credential_path,
        };
        let locations = scriberig_secrets::registry::RegistryLocations {
            user_dir: project_root.join("secrets"),
            system_policy: None,
        };
        locations
            .lock()
            .expect("lock test policy")
            .bind(Binding {
                credential: reference.clone(),
                base_url: endpoint.clone(),
                project_root: project_root.clone(),
                purpose: "model_provider".to_owned(),
                mode: ExecutionMode::Interactive,
            })
            .expect("grant exact connection");
        let model = rpi_ai::Model::new(
            "model-a",
            "model-a",
            rpi_ai::Api::OpenaiResponses,
            "openai",
            &endpoint,
        );
        let raw = format!(
            "[{{\"provider_id\":\"openai\",\"model_id\":\"model-a\",\"credential\":{{\"source\":\"file\",\"path\":{}}}}}]",
            serde_json::to_string(&reference_path(&reference)).expect("encode path")
        );
        let credentials =
            authorized_connection_credential_provider_in(&raw, &model, project_root, locations)
                .expect("valid connection configuration")
                .expect("selected model credential");
        assert_eq!(
            credentials
                .credential(&model.base_url)
                .expect("authorized endpoint")
                .expect("credential")
                .expose(),
            "connection-secret-test-only"
        );
        let request_secret =
            scriberig_agent_runtime::protected_credential::request_secret_for_model(
                credentials.as_ref(),
                &model,
            )
            .expect("upstream protected credential bridge")
            .expect("selected connection request secret");
        let provider = rpi_ai::providers::openai_responses::OpenAiResponsesProvider::with_models_without_env_api_key(
            "openai",
            None,
            reqwest::Client::new(),
            vec![model.clone()],
        );
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("provider runtime");
        let result = runtime.block_on(async {
            provider
                .stream_simple_with_request_secret(
                    &model,
                    &rpi_ai::Context::new(Vec::new()),
                    &rpi_ai::SimpleStreamOptions::default(),
                    &request_secret,
                    true,
                )
                .await
                .result()
                .await
        });
        let (headers, body) = provider_server.join().expect("provider request");
        assert!(
            result.is_ok(),
            "upstream provider should complete the request"
        );
        assert!(
            headers
                .to_ascii_lowercase()
                .contains("authorization: bearer")
        );
        assert!(headers.contains("connection-secret-test-only"));
        assert!(!body.contains("connection-secret-test-only"));
        assert_eq!(
            credentials
                .credential("https://other.example.test/v1")
                .expect_err("other endpoint denied"),
            scriberig_secrets::SecretError::Denied
        );
    }

    /// Returns the indirect file path from the test reference.
    ///
    /// - `reference`: Typed test credential reference.
    ///
    /// The referenced path.
    /// # Returns
    /// The path used by the fixture JSON.
    fn reference_path(reference: &SecretRef) -> &std::path::Path {
        match reference {
            SecretRef::File { path } => path,
            _ => unreachable!("fixture uses a file reference"),
        }
    }

    /// Serves one local Responses request and captures its headers and body.
    ///
    /// # Returns
    /// The endpoint URL and request-capture thread.
    /// # Panics
    /// Panics when the loopback request is malformed or the listener fails.
    fn responses_fixture() -> (String, JoinHandle<(String, String)>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind provider");
        let endpoint = format!("http://{}/v1", listener.local_addr().expect("address"));
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().expect("accept provider request");
            let mut request = Vec::new();
            let mut chunk = [0_u8; 4096];
            let (header_end, content_length) = loop {
                let read = socket.read(&mut chunk).expect("read provider headers");
                assert!(read > 0, "provider closed before request headers");
                request.extend_from_slice(&chunk[..read]);
                if let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..header_end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().expect("content length"))
                        })
                        .expect("request content length");
                    break (header_end, length);
                }
            };
            let body_start = header_end + 4;
            while request.len() < body_start + content_length {
                let read = socket.read(&mut chunk).expect("read provider body");
                assert!(read > 0, "provider closed before full request");
                request.extend_from_slice(&chunk[..read]);
            }
            let headers = String::from_utf8_lossy(&request[..header_end]).into_owned();
            let body = String::from_utf8_lossy(&request[body_start..body_start + content_length])
                .into_owned();
            let response_body = concat!(
                "event: response.completed\r\n",
                "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_connection\",\"status\":\"completed\",\"output\":[]}}\r\n",
                "\r\n",
            );
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            socket
                .write_all(response.as_bytes())
                .expect("write response");
            (headers, body)
        });
        (endpoint, server)
    }
}
