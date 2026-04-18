use crate::api::agile;
use crate::cli::args::GlobalArgs;
use crate::cli::EpicCmd;
use crate::config::JiraConfig;
use crate::error::Result;
use crate::http::HttpClient;
use crate::output::{emit_value, Format};
use std::io::Write;

pub fn dispatch<W: Write>(
    out: &mut W,
    cfg: &JiraConfig,
    client: &HttpClient,
    g: &GlobalArgs,
    cmd: &EpicCmd,
) -> Result<()> {
    match cmd {
        EpicCmd::Get { key } => {
            let v = agile::get_epic(client, key)?;
            let fields = g.field_list();
            emit_value(
                out,
                v,
                &g.output_options_with_renames(
                    Format::Json,
                    fields.as_deref(),
                    Some(&cfg.field_renames),
                ),
            )
        }
        EpicCmd::Issues { key } => {
            let v = agile::epic_issues(client, key)?;
            let fields = g.field_list();
            emit_value(
                out,
                v,
                &g.output_options_with_renames(
                    Format::Json,
                    fields.as_deref(),
                    Some(&cfg.field_renames),
                ),
            )
        }
        EpicCmd::AddIssues { key, issues } => {
            agile::epic_add_issues(client, key, issues)?;
            writeln!(
                out,
                "{}",
                serde_json::json!({"ok": true, "epic": key, "added": issues.len()})
            )?;
            Ok(())
        }
        EpicCmd::RemoveIssues { issues } => {
            agile::epic_remove_issues(client, issues)?;
            writeln!(
                out,
                "{}",
                serde_json::json!({"ok": true, "removed": issues.len()})
            )?;
            Ok(())
        }
    }
}
