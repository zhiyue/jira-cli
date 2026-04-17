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
    /// Parallel bulk operations
    #[command(subcommand)]
    Bulk(BulkCmd),
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
    /// Run a JQL query (POST /rest/api/2/search); streams JSONL
    Search(SearchArgs),
    /// Project operations
    #[command(subcommand)]
    Project(ProjectCmd),
    /// User operations
    #[command(subcommand)]
    User(UserCmd),
    /// Agile: boards
    #[command(subcommand)]
    Board(BoardCmd),
    /// Agile: sprints
    #[command(subcommand)]
    Sprint(SprintCmd),
    /// Agile: epics
    #[command(subcommand)]
    Epic(EpicCmd),
    /// Agile: backlog
    #[command(subcommand)]
    Backlog(BacklogCmd),
    /// Bootstrap a cookie session: POST /rest/auth/1/session.
    /// Reads username/password from JIRA_USER/JIRA_PASSWORD env, falling back to
    /// stdin (line-delimited). Prints the cookie string to stdout.
    #[command(subcommand)]
    Session(SessionCmd),
    /// Emit CLI capability discovery schema (self-describing)
    Schema(SchemaArgs),
}

#[derive(Subcommand, Debug)]
pub enum SessionCmd {
    New,
}

#[derive(clap::Args, Debug)]
pub struct SchemaArgs {
    /// Subcommand name; omit for full tree
    pub subcommand: Option<String>,
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
    /// Comment operations (list, add, update, delete)
    #[command(subcommand)]
    Comment(CommentCmd),
    /// Show available transitions for an issue
    #[command(subcommand)]
    Transitions(TransitionsCmd),
    /// Execute a transition
    Transition(TransitionArgs),
    /// Manage issue links
    #[command(subcommand)]
    Link(LinkCmd),
    /// Issue attachments
    #[command(subcommand)]
    Attachment(AttachmentCmd),
    /// Time-tracking worklogs
    #[command(subcommand)]
    Worklog(WorklogCmd),
    /// Issue watchers
    #[command(subcommand)]
    Watchers(WatchersCmd),
}

#[derive(Subcommand, Debug)]
pub enum CommentCmd {
    /// List comments on an issue
    List { key: String },
    /// Add a comment to an issue
    Add {
        key: String,
        #[arg(short, long)]
        body: String,
    },
    /// Update an existing comment
    Update {
        key: String,
        id: String,
        #[arg(short, long)]
        body: String,
    },
    /// Delete a comment
    Delete { key: String, id: String },
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

#[derive(clap::Args, Debug)]
pub struct SearchArgs {
    pub jql: String,
    /// Comma-separated Jira-side field selector
    #[arg(long = "jira-fields")]
    pub jira_fields: Option<String>,
    #[arg(long)]
    pub expand: Option<String>,
    /// Cap total results emitted (after server-side pagination)
    #[arg(long)]
    pub max: Option<u64>,
    /// Page size sent to server (default 100)
    #[arg(long = "page-size", default_value_t = 100)]
    pub page_size: u64,
}

#[derive(Subcommand, Debug)]
pub enum TransitionsCmd {
    /// List available transitions for an issue
    List { key: String },
}

#[derive(clap::Args, Debug)]
pub struct TransitionArgs {
    pub key: String,
    /// Transition name or id
    #[arg(long)]
    pub to: String,
    /// Optional field updates to send with the transition
    #[arg(long = "set", value_name = "KEY=VALUE")]
    pub set: Vec<String>,
}

#[derive(Subcommand, Debug)]
pub enum LinkCmd {
    /// List links on an issue (derived from issue fields.issuelinks)
    List { key: String },
    /// Add a link. `from` is the outward side (e.g. "blocks").
    Add {
        from: String,
        to: String,
        #[arg(long)]
        r#type: String,
    },
    /// Delete a link by id
    Delete { link_id: String },
}

#[derive(Subcommand, Debug)]
pub enum WorklogCmd {
    List {
        key: String,
    },
    Add {
        key: String,
        /// e.g. "1h 30m"
        #[arg(long)]
        time: String,
        /// ISO 8601 started timestamp
        #[arg(long)]
        started: Option<String>,
        #[arg(long)]
        comment: Option<String>,
    },
    Delete {
        key: String,
        id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum AttachmentCmd {
    /// List attachments on an issue
    List { key: String },
    /// Upload one or more files to an issue
    Upload {
        key: String,
        paths: Vec<std::path::PathBuf>,
    },
    /// Download an attachment by id
    Download {
        attachment_id: String,
        /// Output path (default: original filename; use `-` for stdout)
        #[arg(long)]
        out: Option<String>,
    },
    /// Delete an attachment by id
    Delete { attachment_id: String },
}

#[derive(Subcommand, Debug)]
pub enum WatchersCmd {
    List { key: String },
    Add { key: String, user: String },
    Remove { key: String, user: String },
}

#[derive(Subcommand, Debug)]
pub enum ProjectCmd {
    List,
    Get { key: String },
    Statuses { key: String },
}

#[derive(Subcommand, Debug)]
pub enum UserCmd {
    Get { username: String },
    Search { query: String },
}

#[derive(Subcommand, Debug)]
pub enum BoardCmd {
    /// List boards (optionally filtered by type: scrum, kanban)
    List {
        #[arg(long)]
        r#type: Option<String>,
        #[arg(long)]
        project: Option<String>,
    },
    Get {
        id: u64,
    },
    Backlog {
        id: u64,
    },
}

#[derive(Subcommand, Debug)]
pub enum EpicCmd {
    Get { key: String },
    Issues { key: String },
    AddIssues { key: String, issues: Vec<String> },
    RemoveIssues { issues: Vec<String> },
}

#[derive(Subcommand, Debug)]
pub enum BacklogCmd {
    /// Move issues to backlog (removes them from all future/active sprints)
    Move { keys: Vec<String> },
}

#[derive(Subcommand, Debug)]
pub enum BulkCmd {
    /// Bulk-transition issues from a JSONL file
    Transition {
        #[arg(long)]
        file: String,
        #[arg(long)]
        concurrency: Option<usize>,
    },
    /// Bulk-comment issues from a JSONL file
    Comment {
        #[arg(long)]
        file: String,
        #[arg(long)]
        concurrency: Option<usize>,
    },
}

#[derive(Subcommand, Debug)]
pub enum SprintCmd {
    /// List sprints on a board (optionally filter state)
    List {
        #[arg(long)]
        board: u64,
        /// Comma-separated states: future, active, closed
        #[arg(long)]
        state: Option<String>,
    },
    Get {
        id: u64,
    },
    Create {
        #[arg(long)]
        board: u64,
        #[arg(long)]
        name: String,
        #[arg(long)]
        start: Option<String>,
        #[arg(long)]
        end: Option<String>,
        #[arg(long)]
        goal: Option<String>,
    },
    Update {
        id: u64,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        state: Option<String>,
        #[arg(long)]
        start: Option<String>,
        #[arg(long)]
        end: Option<String>,
        #[arg(long)]
        goal: Option<String>,
    },
    Delete {
        id: u64,
    },
    Issues {
        id: u64,
    },
    Move {
        id: u64,
        keys: Vec<String>,
    },
}
