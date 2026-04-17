use crate::api::comment;
use crate::cli::args::GlobalArgs;
use crate::cli::CommentCmd;
use crate::error::Result;
use crate::http::HttpClient;
use crate::output::{emit_line, emit_value, Format};
use std::io::Write;

pub fn dispatch<W: Write>(
    out: &mut W,
    client: &HttpClient,
    g: &GlobalArgs,
    cmd: &CommentCmd,
) -> Result<()> {
    match cmd {
        CommentCmd::List { key } => list(out, client, g, key),
        CommentCmd::Add { key, body } => {
            let v = comment::add(client, key, body)?;
            writeln!(out, "{}", serde_json::json!({"ok": true, "data": v}))?;
            Ok(())
        }
        CommentCmd::Update { key, id, body } => {
            let v = comment::update(client, key, id, body)?;
            writeln!(out, "{}", serde_json::json!({"ok": true, "data": v}))?;
            Ok(())
        }
        CommentCmd::Delete { key, id } => {
            comment::delete(client, key, id)?;
            writeln!(out, "{}", serde_json::json!({"ok": true, "deleted": id}))?;
            Ok(())
        }
    }
}

fn list<W: Write>(out: &mut W, client: &HttpClient, g: &GlobalArgs, key: &str) -> Result<()> {
    let page = comment::list(client, key)?;
    let fields = g.field_list();
    let opts = g.output_options(Format::Jsonl, fields.as_deref());
    for c in &page.comments {
        emit_value(out, c.clone(), &opts)?;
    }
    emit_line(
        out,
        &serde_json::json!({"summary": {"count": page.comments.len(), "total": page.total}}),
    )
}
