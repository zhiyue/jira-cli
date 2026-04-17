use crate::api::user;
use crate::cli::args::GlobalArgs;
use crate::cli::UserCmd;
use crate::error::Result;
use crate::http::HttpClient;
use crate::output::{emit_line, emit_value, Format};
use std::io::Write;

pub fn dispatch<W: Write>(
    out: &mut W,
    client: &HttpClient,
    g: &GlobalArgs,
    cmd: &UserCmd,
) -> Result<()> {
    match cmd {
        UserCmd::Get { username } => {
            let v = user::get(client, username)?;
            emit_value(out, v, &g.output_options(Format::Json, None))
        }
        UserCmd::Search { query } => {
            let items = user::search(client, query)?;
            let opts = g.output_options(Format::Jsonl, None);
            for u in &items {
                emit_value(out, u.clone(), &opts)?;
            }
            emit_line(out, &serde_json::json!({"summary": {"count": items.len()}}))
        }
    }
}
