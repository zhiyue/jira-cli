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
    /// Field metadata operations
    #[command(subcommand)]
    Field(FieldCmd),
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
    /// Create a new issue (POST /issue)
    Create(IssueCreate),
    /// Update an existing issue (PUT /issue/{key})
    Update(IssueUpdate),
    /// Delete an issue (DELETE /issue/{key})
    Delete(IssueDelete),
    /// Assign an issue to a user or unassign (PUT /issue/{key}/assignee)
    Assign(IssueAssign),
    /// Bulk-create issues (POST /issue/bulk). Input is a JSON array of {fields:{...}} objects.
    BulkCreate(IssueBulkCreate),
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

#[derive(clap::Args, Debug)]
pub struct IssueCreate {
    /// Project key, e.g. MGX
    #[arg(short, long)]
    pub project: String,
    /// Issue type name or id (e.g. Task, Bug)
    #[arg(short = 't', long = "type")]
    pub issue_type: String,
    /// Summary (required)
    #[arg(short, long)]
    pub summary: String,
    /// Repeatable KEY=VALUE. VALUE can be scalar, JSON literal, @file, or @-.
    #[arg(long = "set", value_name = "KEY=VALUE")]
    pub set: Vec<String>,
}

#[derive(clap::Args, Debug)]
pub struct IssueUpdate {
    pub key: String,
    #[arg(long = "set", value_name = "KEY=VALUE", required = true)]
    pub set: Vec<String>,
}

#[derive(clap::Args, Debug)]
pub struct IssueDelete {
    pub key: String,
    /// Required confirmation (safety gate)
    #[arg(long)]
    pub yes: bool,
}

#[derive(clap::Args, Debug)]
pub struct IssueAssign {
    pub key: String,
    /// Assignee username; omit or pass --unassign to clear
    #[arg(long)]
    pub user: Option<String>,
    #[arg(long, conflicts_with = "user")]
    pub unassign: bool,
}

#[derive(clap::Args, Debug)]
pub struct IssueBulkCreate {
    /// Path to JSON array file, or `-` for stdin
    #[arg(long = "from-file", value_name = "PATH")]
    pub from_file: String,
}

#[derive(Subcommand, Debug)]
pub enum FieldCmd {
    /// List all fields (standard + custom) via GET /rest/api/2/field
    List,
    /// Resolve a field display name to its id (e.g. "Story Points" → customfield_10020)
    Resolve(FieldResolveArgs),
}

#[derive(clap::Args, Debug)]
pub struct FieldResolveArgs {
    pub name: String,
}
