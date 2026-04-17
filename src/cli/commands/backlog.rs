use crate::api::agile;
use crate::cli::args::GlobalArgs;
use crate::cli::BacklogCmd;
use crate::error::Result;
use crate::http::HttpClient;
use std::io::Write;

pub fn dispatch<W: Write>(
    out: &mut W,
    client: &HttpClient,
    _g: &GlobalArgs,
    cmd: &BacklogCmd,
) -> Result<()> {
    match cmd {
        BacklogCmd::Move { keys } => {
            if keys.is_empty() {
                return Err(crate::error::Error::Usage(
                    "backlog move requires at least one issue key".into(),
                ));
            }
            agile::backlog_move(client, keys)?;
            writeln!(
                out,
                "{}",
                serde_json::json!({"ok": true, "moved": keys.len()})
            )?;
            Ok(())
        }
    }
}
