use crate::api::watchers;
use crate::cli::args::GlobalArgs;
use crate::cli::WatchersCmd;
use crate::error::Result;
use crate::http::HttpClient;
use crate::output::{emit_value, Format};
use std::io::Write;

pub fn dispatch<W: Write>(
    out: &mut W,
    client: &HttpClient,
    g: &GlobalArgs,
    cmd: &WatchersCmd,
) -> Result<()> {
    match cmd {
        WatchersCmd::List { key } => {
            let v = watchers::list(client, key)?;
            let fields = g.field_list();
            emit_value(out, v, &g.output_options(Format::Json, fields.as_deref()))
        }
        WatchersCmd::Add { key, user } => {
            watchers::add(client, key, user)?;
            writeln!(
                out,
                "{}",
                serde_json::json!({"ok": true, "key": key, "added": user})
            )?;
            Ok(())
        }
        WatchersCmd::Remove { key, user } => {
            watchers::remove(client, key, user)?;
            writeln!(
                out,
                "{}",
                serde_json::json!({"ok": true, "key": key, "removed": user})
            )?;
            Ok(())
        }
    }
}
