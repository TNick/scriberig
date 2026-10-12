//! Composes CLI commands that consume the public remote protocol client.

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Creates a semantic client for a public protocol endpoint.
///
/// # Parameters
/// - `endpoint`: HTTP endpoint selected by the CLI.
///
/// # Returns
/// A protocol client over the endpoint.
///
/// # Errors
/// Returns an endpoint parsing or transport setup error.
fn client(endpoint: &str) -> Result<scriberig_client::Client<scriberig_client::HttpTransport>> {
    Ok(scriberig_client::Client::new(
        scriberig_client::HttpTransport::from_endpoint(endpoint)?,
    ))
}

/// Prints capabilities exposed by a remote application.
///
/// # Parameters
/// - `super`: Module namespace containing the command composition helpers.
/// - `args`: Parsed remote capability options.
///
/// # Returns
/// Unit after printing the capability response.
///
/// # Errors
/// Returns a client or protocol failure.
pub(super) fn capabilities(args: &scriberig_cli::RemoteArgs) -> Result<()> {
    let response = client(&args.connect)?.capabilities()?;
    if args.json {
        println!("{}", scriberig_cli::output::render_json(&response));
    } else {
        for capability in response.capabilities {
            println!(
                "{}\t{}\t{}",
                capability.id, capability.available, capability.description
            );
        }
    }
    Ok(())
}

/// Executes one remote operation lifecycle command.
///
/// # Parameters
/// - `super`: Module namespace containing the command composition helpers.
/// - `args`: Parsed operation command.
///
/// # Returns
/// Unit after printing the protocol response.
///
/// # Errors
/// Returns a client or protocol failure.
pub(super) fn operation(args: scriberig_cli::OperationArgs) -> Result<()> {
    match args.command {
        scriberig_cli::OperationCommand::Status { operation, remote } => {
            let response = client(&remote.connect)?.operation_status(operation)?;
            println!("{}", scriberig_cli::output::render_json(&response));
        }
        scriberig_cli::OperationCommand::Cancel { operation, remote } => {
            let response = client(&remote.connect)?.cancel_operation(operation)?;
            println!("{}", scriberig_cli::output::render_json(&response));
        }
    }
    Ok(())
}

/// Reads buffered lifecycle or streaming events.
///
/// # Parameters
/// - `super`: Module namespace containing the command composition helpers.
/// - `args`: Parsed event filter and cursor.
///
/// # Returns
/// Unit after printing the event response.
///
/// # Errors
/// Returns a client or protocol failure.
pub(super) fn events(args: scriberig_cli::EventsArgs) -> Result<()> {
    let events = if let Some(after) = args.after {
        client(&args.remote.connect)?.subscribe_after(args.event_kinds, Some(after))?
    } else {
        client(&args.remote.connect)?.subscribe(args.event_kinds)?
    };
    println!("{}", scriberig_cli::output::render_json(&events));
    Ok(())
}

/// Reads a durable session tree or branch activity view.
///
/// # Parameters
/// - `super`: Module namespace containing the command composition helpers.
/// - `args`: Parsed session read command.
///
/// # Returns
/// Unit after printing the session response.
///
/// # Errors
/// Returns a client or protocol failure.
pub(super) fn session(args: scriberig_cli::SessionArgs) -> Result<()> {
    match args.command {
        scriberig_cli::SessionCommand::Tree { session, remote } => {
            let response = client(&remote.connect)?.read_session(session)?;
            println!("{}", scriberig_cli::output::render_json(&response));
        }
        scriberig_cli::SessionCommand::Activity {
            session,
            branch,
            remote,
        } => {
            let response = client(&remote.connect)?.read_session_branch(session, branch)?;
            println!("{}", scriberig_cli::output::render_json(&response));
        }
    }
    Ok(())
}

/// Continues one remote agent branch.
///
/// # Parameters
/// - `super`: Module namespace containing the command composition helpers.
/// - `args`: Parsed session, branch, prompt, and endpoint.
///
/// # Returns
/// Unit after printing the accepted operation handle.
///
/// # Errors
/// Returns a client or protocol failure.
pub(super) fn agent_continue(args: scriberig_cli::AgentContinueArgs) -> Result<()> {
    let response = client(&args.remote.connect)?.continue_agent_with_handles(
        args.session,
        args.branch,
        args.prompt,
    )?;
    if args.remote.json {
        println!("{}", scriberig_cli::output::render_json(&response));
    } else {
        println!("{}", response.operation.operation.reference.id);
    }
    Ok(())
}
