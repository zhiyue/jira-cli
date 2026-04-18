use crate::api::worklog;
use crate::cli::args::GlobalArgs;
use crate::cli::WorklogCmd;
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
    cmd: &WorklogCmd,
) -> Result<()> {
    match cmd {
        WorklogCmd::List {
            key,
            max,
            start_at,
            page_size,
        } => {
            let params = crate::api::paging::PageParams {
                start_at: *start_at,
                page_size: *page_size,
                max: *max,
            };
            let mut iter = worklog::list_paged(client, key, params);
            let renames = cfg.effective_renames(client)?;
            let fields = g.field_list();
            let opts =
                g.output_options_with_renames(Format::Jsonl, fields.as_deref(), Some(&renames));
            let mut count = 0u64;
            for next in iter.by_ref() {
                let w = next?;
                emit_value(out, w, &opts)?;
                count += 1;
            }
            emit_line(
                out,
                &serde_json::json!({"summary": {"count": count, "total": iter.total()}}),
            )
        }
        WorklogCmd::Add {
            key,
            time,
            started,
            comment,
        } => {
            let renames = cfg.effective_renames(client)?;
            let mut v = serde_json::json!({"ok": true, "data": worklog::add(client, key, time, started.as_deref(), comment.as_deref())?});
            crate::output::rename_keys(&mut v, &renames);
            writeln!(out, "{v}")?;
            Ok(())
        }
        WorklogCmd::Delete { key, id } => {
            worklog::delete(client, key, id)?;
            writeln!(out, "{}", serde_json::json!({"ok": true, "deleted": id}))?;
            Ok(())
        }
    }
}
