use crate::api::link;
use crate::cli::args::GlobalArgs;
use crate::cli::LinkCmd;
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
    cmd: &LinkCmd,
) -> Result<()> {
    match cmd {
        LinkCmd::List { key } => {
            let items = link::list_for_issue(client, key)?;
            let fields = g.field_list();
            let opts = g.output_options_with_renames(
                Format::Jsonl,
                fields.as_deref(),
                Some(&cfg.field_renames),
            );
            for l in &items {
                emit_value(out, l.clone(), &opts)?;
            }
            emit_line(out, &serde_json::json!({"summary": {"count": items.len()}}))
        }
        LinkCmd::Add { from, to, r#type } => {
            link::create(client, from, to, r#type)?;
            writeln!(
                out,
                "{}",
                serde_json::json!({"ok": true, "from": from, "to": to, "type": r#type})
            )?;
            Ok(())
        }
        LinkCmd::Delete { link_id } => {
            link::delete(client, link_id)?;
            writeln!(
                out,
                "{}",
                serde_json::json!({"ok": true, "deleted": link_id})
            )?;
            Ok(())
        }
    }
}
