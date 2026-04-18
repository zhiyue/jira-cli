use crate::api::agile;
use crate::cli::args::GlobalArgs;
use crate::cli::SprintCmd;
use crate::error::Result;
use crate::http::HttpClient;
use crate::output::{emit_line, emit_value, Format};
use std::io::Write;

pub fn dispatch<W: Write>(
    out: &mut W,
    client: &HttpClient,
    g: &GlobalArgs,
    cmd: &SprintCmd,
) -> Result<()> {
    match cmd {
        SprintCmd::List { board, state } => {
            let states: Vec<&str> = state
                .as_deref()
                .map(|s| {
                    s.split(',')
                        .map(str::trim)
                        .filter(|x| !x.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            let page = agile::list_sprints(client, *board, &states)?;
            let fields = g.field_list();
            let opts = g.output_options(Format::Jsonl, fields.as_deref());
            for s in &page.values {
                emit_value(out, s.clone(), &opts)?;
            }
            emit_line(
                out,
                &serde_json::json!({"summary": {"count": page.values.len(), "total": page.total, "isLast": page.is_last}}),
            )
        }
        SprintCmd::Get { id } => {
            let v = agile::get_sprint(client, *id)?;
            let fields = g.field_list();
            emit_value(out, v, &g.output_options(Format::Json, fields.as_deref()))
        }
        SprintCmd::Create {
            board,
            name,
            start,
            end,
            goal,
        } => {
            let v = agile::create_sprint(
                client,
                *board,
                name,
                start.as_deref(),
                end.as_deref(),
                goal.as_deref(),
            )?;
            writeln!(out, "{}", serde_json::json!({"ok": true, "data": v}))?;
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
            let v = agile::update_sprint(client, *id, &serde_json::Value::Object(body))?;
            writeln!(out, "{}", serde_json::json!({"ok": true, "data": v}))?;
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
        SprintCmd::Issues { id } => {
            let v = agile::sprint_issues(client, *id)?;
            let fields = g.field_list();
            emit_value(out, v, &g.output_options(Format::Json, fields.as_deref()))
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
