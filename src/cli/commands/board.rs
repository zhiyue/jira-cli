use crate::api::agile;
use crate::cli::args::GlobalArgs;
use crate::cli::BoardCmd;
use crate::config::JiraConfig;
use crate::error::Result;
use crate::http::HttpClient;
use crate::output::{emit_line, emit_value, Format};
use std::io::Write;

pub fn dispatch<W: Write>(
    out: &mut W,
    cfg: &JiraConfig,
    client: &HttpClient,
    g: &GlobalArgs,
    cmd: &BoardCmd,
) -> Result<()> {
    match cmd {
        BoardCmd::List { r#type, project } => {
            let page = agile::list_boards(client, r#type.as_deref(), project.as_deref())?;
            let fields = g.field_list();
            let opts = g.output_options_with_renames(
                Format::Jsonl,
                fields.as_deref(),
                Some(&cfg.field_renames),
            );
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
            let fields = g.field_list();
            emit_value(
                out,
                v,
                &g.output_options_with_renames(
                    Format::Json,
                    fields.as_deref(),
                    Some(&cfg.field_renames),
                ),
            )
        }
        BoardCmd::Backlog { id } => {
            let v = agile::board_backlog(client, *id)?;
            let fields = g.field_list();
            emit_value(
                out,
                v,
                &g.output_options_with_renames(
                    Format::Json,
                    fields.as_deref(),
                    Some(&cfg.field_renames),
                ),
            )
        }
    }
}
