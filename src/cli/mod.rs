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
    /// Effective configuration view
    #[command(subcommand)]
    Config(ConfigCmd),
    /// Issue-level operations
    #[command(subcommand)]
    Issue(IssueCmd),
}

#[derive(Subcommand, Debug)]
pub enum ConfigCmd {
    /// Print the parsed configuration
    Show,
}

#[derive(Subcommand, Debug)]
pub enum IssueCmd {
    /// GET /rest/api/2/issue/{key}
    Get(IssueGet),
}

#[derive(clap::Args, Debug)]
pub struct IssueGet {
    /// Issue key (e.g. MGX-42)
    pub key: String,
    /// Comma-separated fields to include (maps to Jira's `fields` query param)
    #[arg(long)]
    pub jira_fields: Option<String>,
    /// Comma-separated `expand` values (e.g. changelog,renderedFields)
    #[arg(long)]
    pub expand: Option<String>,
}
