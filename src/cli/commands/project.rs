use crate::api::project;
use crate::cli::args::GlobalArgs;
use crate::cli::ProjectCmd;
use crate::error::Result;
use crate::http::HttpClient;
use crate::output::{emit_line, emit_value, Format};
use std::io::Write;

pub fn dispatch<W: Write>(
    out: &mut W,
    client: &HttpClient,
    g: &GlobalArgs,
    cmd: &ProjectCmd,
) -> Result<()> {
    match cmd {
        ProjectCmd::List => {
            let items = project::list(client)?;
            let fields = g.field_list();
            let opts = g.output_options(Format::Jsonl, fields.as_deref());
            for p in &items {
                emit_value(out, p.clone(), &opts)?;
            }
            emit_line(out, &serde_json::json!({"summary": {"count": items.len()}}))
        }
        ProjectCmd::Get { key } => {
            let v = project::get(client, key)?;
            let fields = g.field_list();
            emit_value(out, v, &g.output_options(Format::Json, fields.as_deref()))
        }
        ProjectCmd::Statuses { key } => {
            let v = project::statuses(client, key)?;
            let fields = g.field_list();
            emit_value(out, v, &g.output_options(Format::Json, fields.as_deref()))
        }
    }
}
