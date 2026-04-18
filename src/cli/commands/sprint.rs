use crate::api::agile;
use crate::cli::args::GlobalArgs;
use crate::cli::SprintCmd;
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
    cmd: &SprintCmd,
) -> Result<()> {
    match cmd {
        SprintCmd::List {
            board,
            state,
            max,
            start_at,
            page_size,
        } => {
            let states: Vec<&str> = state
                .as_deref()
                .map(|s| {
                    s.split(',')
                        .map(str::trim)
                        .filter(|x| !x.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            let params = crate::api::paging::PageParams {
                start_at: *start_at,
                page_size: *page_size,
                max: *max,
            };
            let mut iter = agile::list_sprints_paged(client, *board, &states, params);
            let renames = cfg.effective_renames(client)?;
            let fields = g.field_list();
            let opts =
                g.output_options_with_renames(Format::Jsonl, fields.as_deref(), Some(&renames));
            let mut count = 0u64;
            for next in iter.by_ref() {
                let s = next?;
                emit_value(out, s, &opts)?;
                count += 1;
            }
            emit_line(
                out,
                &serde_json::json!({"summary": {"count": count, "total": iter.total()}}),
            )
        }
        SprintCmd::Get { id } => {
            let v = agile::get_sprint(client, *id)?;
            let fields = g.field_list();
            let renames = cfg.effective_renames(client)?;
            emit_value(
                out,
                v,
                &g.output_options_with_renames(Format::Json, fields.as_deref(), Some(&renames)),
            )
        }
        SprintCmd::Create {
            board,
            name,
            start,
            end,
            goal,
        } => {
            let renames = cfg.effective_renames(client)?;
            let mut v = serde_json::json!({"ok": true, "data": agile::create_sprint(
                client,
                *board,
                name,
                start.as_deref(),
                end.as_deref(),
                goal.as_deref(),
            )?});
            crate::output::rename_keys(&mut v, &renames);
            writeln!(out, "{v}")?;
            Ok(())
        }
        SprintCmd::Update {
            id,
            name,
            state,
            start,
            end,
            goal,
        } => {
            let mut body = serde_json::Map::new();
            if let Some(n) = name {
                body.insert("name".into(), serde_json::json!(n));
            }
            if let Some(s) = state {
                body.insert("state".into(), serde_json::json!(s));
            }
            if let Some(s) = start {
                body.insert("startDate".into(), serde_json::json!(s));
            }
            if let Some(e) = end {
                body.insert("endDate".into(), serde_json::json!(e));
            }
            if let Some(g) = goal {
                body.insert("goal".into(), serde_json::json!(g));
            }
            let renames = cfg.effective_renames(client)?;
            let mut v = serde_json::json!({"ok": true, "data": agile::update_sprint(client, *id, &serde_json::Value::Object(body))?});
            crate::output::rename_keys(&mut v, &renames);
            writeln!(out, "{v}")?;
            Ok(())
        }
        SprintCmd::Delete { id, yes } => {
            if !*yes {
                return Err(crate::error::Error::Usage(
                    "sprint delete requires --yes to confirm".into(),
                ));
            }
            agile::delete_sprint(client, *id)?;
            writeln!(out, "{}", serde_json::json!({"ok": true, "deleted": id}))?;
            Ok(())
        }
        SprintCmd::Issues {
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
            let mut iter = agile::sprint_issues_paged(client, *id, params);
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
        SprintCmd::Move { id, keys } => {
            agile::move_issues_to_sprint(client, *id, keys)?;
            writeln!(
                out,
                "{}",
                serde_json::json!({"ok": true, "id": id, "moved": keys.len()})
            )?;
            Ok(())
        }
    }
}
