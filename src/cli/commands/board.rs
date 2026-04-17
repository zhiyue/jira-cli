use crate::api::agile;
use crate::cli::args::GlobalArgs;
use crate::cli::BoardCmd;
use crate::error::Result;
use crate::http::HttpClient;
use crate::output::{emit_line, emit_value, Format};
use std::io::Write;

pub fn dispatch<W: Write>(
    out: &mut W,
    client: &HttpClient,
    g: &GlobalArgs,
    cmd: &BoardCmd,
) -> Result<()> {
    match cmd {
        BoardCmd::List { r#type, project } => {
            let page = agile::list_boards(client, r#type.as_deref(), project.as_deref())?;
            let opts = g.output_options(Format::Jsonl, None);
            for b in &page.values {
                emit_value(out, b.clone(), &opts)?;
            }
            emit_line(
                out,
                &serde_json::json!({"summary": {"count": page.values.len(), "total": page.total, "isLast": page.is_last}}),
            )
        }
        BoardCmd::Get { id } => {
            let v = agile::get_board(client, *id)?;
            emit_value(out, v, &g.output_options(Format::Json, None))
        }
        BoardCmd::Backlog { id } => {
            let v = agile::board_backlog(client, *id)?;
            emit_value(out, v, &g.output_options(Format::Json, None))
        }
    }
}
