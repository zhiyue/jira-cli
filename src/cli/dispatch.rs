//! Match subcommand → run.

use crate::cli::{commands, Cli, Command, SessionCmd};
use crate::config::JiraConfig;
use crate::error::Result;
use crate::http::HttpClient;
use std::io::Write;

pub fn run<W: Write>(out: &mut W, cfg: &JiraConfig, client: &HttpClient, cli: &Cli) -> Result<()> {
    match &cli.cmd {
        Command::Bulk(sub) => commands::bulk::dispatch(out, cfg, client, &cli.global, sub),
        Command::Ping => commands::meta::ping(out, client, &cli.global),
        Command::Whoami => commands::meta::whoami(out, client, &cli.global),
        Command::Config(sub) => commands::meta::config(out, cfg, &cli.global, sub),
        Command::Issue(sub) => commands::issue::dispatch(out, cfg, client, &cli.global, sub),
        Command::Field(sub) => commands::field::dispatch(out, cfg, client, &cli.global, sub),
        Command::Search(a) => commands::search::run(out, client, &cli.global, a),
        Command::Project(sub) => commands::project::dispatch(out, client, &cli.global, sub),
        Command::User(sub) => commands::user::dispatch(out, client, &cli.global, sub),
        Command::Board(sub) => commands::board::dispatch(out, client, &cli.global, sub),
        Command::Sprint(sub) => commands::sprint::dispatch(out, client, &cli.global, sub),
        Command::Epic(sub) => commands::epic::dispatch(out, client, &cli.global, sub),
        Command::Backlog(sub) => commands::backlog::dispatch(out, client, &cli.global, sub),
        Command::Session(SessionCmd::New) => commands::meta::session_new(out, cfg),
        Command::Schema(a) => commands::schema::run(out, a, cli.global.pretty),
    }
}
