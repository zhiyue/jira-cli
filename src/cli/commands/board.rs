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
        BoardCmd::List {
            r#type,
            project,
            max,
            start_at,
            page_size,
        } => {
            let params = crate::api::paging::PageParams {
                start_at: *start_at,
                page_size: *page_size,
                max: *max,
            };
            let mut iter =
                agile::list_boards_paged(client, r#type.as_deref(), project.as_deref(), params);
            let renames = cfg.effective_renames(client)?;
            let fields = g.field_list();
            let opts =
                g.output_options_with_renames(Format::Jsonl, fields.as_deref(), Some(&renames));
            let mut count = 0u64;
            for next in iter.by_ref() {
                let b = next?;
                emit_value(out, b, &opts)?;
                count += 1;
            }
            emit_line(
                out,
                &serde_json::json!({"summary": {"count": count, "total": iter.total()}}),
            )
        }
        BoardCmd::Get { id } => {
            let v = agile::get_board(client, *id)?;
            let fields = g.field_list();
            let renames = cfg.effective_renames(client)?;
            emit_value(
                out,
                v,
                &g.output_options_with_renames(Format::Json, fields.as_deref(), Some(&renames)),
            )
        }
        BoardCmd::Backlog {
            id,
            max,
            start_at,
            page_size,
        } => {
            let params = crate::api::paging::PageParams {
                start_at: *start_at,
                page_size: *page_size,
                max: *max,
            };
            let mut iter = agile::board_backlog_paged(client, *id, params);
            let renames = cfg.effective_renames(client)?;
            let fields = g.field_list();
            let opts =
                g.output_options_with_renames(Format::Jsonl, fields.as_deref(), Some(&renames));
            let mut count = 0u64;
            for next in iter.by_ref() {
                let issue = next?;
                emit_value(out, issue, &opts)?;
                count += 1;
            }
            emit_line(
                out,
                &serde_json::json!({"summary": {"count": count, "total": iter.total()}}),
            )
        }
    }
}
