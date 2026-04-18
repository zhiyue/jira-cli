use crate::api::comment;
use crate::cli::args::GlobalArgs;
use crate::cli::CommentCmd;
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
    cmd: &CommentCmd,
) -> Result<()> {
    match cmd {
        CommentCmd::List { key } => list(out, cfg, client, g, key),
        CommentCmd::Add { key, body } => {
            let renames = cfg.effective_renames(client)?;
            let mut v = serde_json::json!({"ok": true, "data": comment::add(client, key, body)?});
            crate::output::rename_keys(&mut v, &renames);
            writeln!(out, "{v}")?;
            Ok(())
        }
        CommentCmd::Update { key, id, body } => {
            let renames = cfg.effective_renames(client)?;
            let mut v =
                serde_json::json!({"ok": true, "data": comment::update(client, key, id, body)?});
            crate::output::rename_keys(&mut v, &renames);
            writeln!(out, "{v}")?;
            Ok(())
        }
        CommentCmd::Delete { key, id } => {
            comment::delete(client, key, id)?;
            writeln!(out, "{}", serde_json::json!({"ok": true, "deleted": id}))?;
            Ok(())
        }
    }
}

fn list<W: Write>(
    out: &mut W,
    cfg: &JiraConfig,
    client: &HttpClient,
    g: &GlobalArgs,
    key: &str,
) -> Result<()> {
    let page = comment::list(client, key)?;
    let fields = g.field_list();
    let renames = cfg.effective_renames(client)?;
    let opts = g.output_options_with_renames(Format::Jsonl, fields.as_deref(), Some(&renames));
    for c in &page.comments {
        emit_value(out, c.clone(), &opts)?;
    }
    emit_line(
        out,
        &serde_json::json!({"summary": {"count": page.comments.len(), "total": page.total}}),
    )
}
