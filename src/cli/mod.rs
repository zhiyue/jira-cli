//! CLI root.

pub mod args;
pub mod commands;
pub mod dispatch;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "jira-cli",
    author,
    version,
    about = "Agent-first CLI for legacy Jira Server 8.13.5"
)]
pub struct Cli {
    #[command(flatten)]
    pub global: args::GlobalArgs,

    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Connectivity probe: GET /rest/api/2/serverInfo
    Ping,
    /// Current authenticated user: GET /rest/api/2/myself
    Whoami,
    /// Print effective configuration (secrets redacted)
    #[command(name = "config", subcommand)]
    Config(ConfigCmd),
    // Further subcommands are added by later tasks.
}

#[derive(Subcommand, Debug)]
pub enum ConfigCmd {
    /// Print the parsed configuration
    Show,
}
