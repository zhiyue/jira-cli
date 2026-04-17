use crate::api::attachment;
use crate::cli::args::GlobalArgs;
use crate::cli::AttachmentCmd;
use crate::error::{Error, Result};
use crate::http::HttpClient;
use crate::output::{emit_line, emit_value, Format};
use std::io::Write;

pub fn dispatch<W: Write>(
    out: &mut W,
    client: &HttpClient,
    g: &GlobalArgs,
    cmd: &AttachmentCmd,
) -> Result<()> {
    match cmd {
        AttachmentCmd::List { key } => {
            let items = attachment::list_for_issue(client, key)?;
            let opts = g.output_options(Format::Jsonl, None);
            for a in &items {
                emit_value(out, a.clone(), &opts)?;
            }
            emit_line(out, &serde_json::json!({"summary": {"count": items.len()}}))
        }
        AttachmentCmd::Upload { key, paths } => {
            if paths.is_empty() {
                return Err(Error::Usage(
                    "upload requires at least one file path".into(),
                ));
            }
            let v = attachment::upload(client, key, paths)?;
            writeln!(out, "{}", serde_json::json!({"ok": true, "data": v}))?;
            Ok(())
        }
        AttachmentCmd::Download {
            attachment_id,
            out: out_path,
        } => {
            let meta = attachment::meta(client, attachment_id)?;
            let content_url = meta["content"].as_str().ok_or_else(|| {
                Error::Api(crate::error::ApiErrorBody {
                    status: 200,
                    error_messages: vec!["attachment metadata missing `content` URL".into()],
                    errors: Default::default(),
                    request_id: None,
                })
            })?;
            let bytes = attachment::download(client, content_url)?;

            let target = match out_path.as_deref() {
                Some("-") => None,
                Some(p) => Some(p.to_string()),
                None => meta["filename"].as_str().map(String::from),
            };

            match target {
                None => {
                    out.write_all(&bytes)?;
                }
                Some(path) => {
                    std::fs::write(&path, &bytes)?;
                    writeln!(
                        out,
                        "{}",
                        serde_json::json!({"ok": true, "path": path, "bytes": bytes.len()})
                    )?;
                }
            }
            Ok(())
        }
        AttachmentCmd::Delete { attachment_id } => {
            attachment::delete(client, attachment_id)?;
            writeln!(
                out,
                "{}",
                serde_json::json!({"ok": true, "deleted": attachment_id})
            )?;
            Ok(())
        }
    }
}
