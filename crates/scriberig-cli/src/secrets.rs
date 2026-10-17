//! Defines local secret-management command arguments without reading secrets.

use crate::RunFormat;
use clap::{Args, Subcommand};
use std::path::PathBuf;

/// Selects a local administrative secret operation.
#[derive(Debug, Args)]
pub struct SecretsArgs {
    /// Requested lifecycle or binding operation.
    #[command(subcommand)]
    pub command: SecretsCommand,
}

/// Describes lifecycle commands that never accept secret values as arguments.
#[derive(Debug, Subcommand)]
pub enum SecretsCommand {
    /// Store a credential from a hidden prompt or explicit stdin.
    Set {
        /// Managed OS-store alias.
        name: String,
        /// Read bounded input from stdin instead of a hidden prompt.
        #[arg(long)]
        stdin: bool,
        /// Explicitly replace an existing item.
        #[arg(long)]
        replace: bool,
    },
    /// List managed aliases without retrieving their values.
    List {
        /// Output representation.
        #[arg(long, value_enum, default_value_t = RunFormat::Human)]
        format: RunFormat,
    },
    /// Inspect metadata availability without testing a live provider.
    Status {
        /// Managed OS-store alias.
        name: String,
        /// Output representation.
        #[arg(long, value_enum, default_value_t = RunFormat::Human)]
        format: RunFormat,
    },
    /// Remove all bindings before deleting a managed credential.
    Delete {
        /// Managed OS-store alias.
        name: String,
    },
    /// Authorize an exact provider destination and project.
    Bind(BindingArgs),
    /// Remove an exact provider destination/project authorization.
    Unbind(BindingArgs),
}

/// Identifies an exact administrative credential-use binding.
#[derive(Debug, Args)]
pub struct BindingArgs {
    /// Alias or typed environment/file reference; never a secret value.
    pub name: String,
    /// Exact provider base URL.
    #[arg(long)]
    pub base_url: String,
    /// Existing project directory to authorize.
    #[arg(long)]
    pub project_root: PathBuf,
    /// Grant service execution instead of local interactive execution.
    #[arg(long)]
    pub service: bool,
}
