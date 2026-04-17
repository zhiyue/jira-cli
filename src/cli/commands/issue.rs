//! Issue CLI commands.

use crate::api::issue;
use crate::cli::args::GlobalArgs;
use crate::cli::IssueCmd;
use crate::error::Result;
use crate::http::HttpClient;
use crate::output::{emit_value, Format};
use std::io::Write;

pub fn dispatch<W: Write>(
    out: &mut W,
    client: &HttpClient,
    g: &GlobalArgs,
    cmd: &IssueCmd,
) -> Result<()> {
    match cmd {
        IssueCmd::Get(args) => get(out, client, g, args),
    }
}

fn get<W: Write>(
    out: &mut W,
    client: &HttpClient,
    g: &GlobalArgs,
    args: &crate::cli::IssueGet,
) -> Result<()> {
    let opts = issue::GetOpts {
        fields: split_csv(args.jira_fields.as_deref()),
        expand: split_csv(args.expand.as_deref()),
    };
    let v = issue::get(client, &args.key, &opts)?;
    let fields = g.field_list();
    emit_value(out, v, &g.output_options(Format::Json, fields.as_deref()))
}

fn split_csv(raw: Option<&str>) -> Vec<String> {
    raw.map(|s| {
        s.split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect::<Vec<_>>()
    })
    .unwrap_or_default()
}
