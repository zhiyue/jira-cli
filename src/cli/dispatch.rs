//! Match subcommand → run.

use crate::cli::{commands, Cli, Command};
use crate::config::JiraConfig;
use crate::error::Result;
use crate::http::HttpClient;
use std::io::Write;

pub fn run<W: Write>(out: &mut W, cfg: &JiraConfig, client: &HttpClient, cli: &Cli) -> Result<()> {
    match &cli.cmd {
        Command::Ping => commands::meta::ping(out, client, &cli.global),
        Command::Whoami => commands::meta::whoami(out, client, &cli.global),
        Command::Config(sub) => commands::meta::config(out, cfg, &cli.global, sub),
        Command::Issue(sub) => commands::issue::dispatch(out, client, &cli.global, sub),
        Command::Field(sub) => commands::field::dispatch(out, client, &cli.global, sub),
        Command::Search(a) => commands::search::run(out, client, &cli.global, a),
        Command::Project(sub) => commands::project::dispatch(out, client, &cli.global, sub),
        Command::User(sub) => commands::user::dispatch(out, client, &cli.global, sub),
        Command::Board(sub) => commands::board::dispatch(out, client, &cli.global, sub),
        Command::Sprint(sub) => commands::sprint::dispatch(out, client, &cli.global, sub),
    }
}
